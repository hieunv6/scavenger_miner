use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, ACCEPT};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use std::time::Instant;
use std::io::{self, Write};

// Import AshMaize từ dependency
use ashmaize::{hash, Rom, RomGenerationType};

const BASE_URL: &str = "https://scavenger.prod.gd.midnighttge.io";

// ==================== API STRUCTURES ====================

#[derive(Debug, Deserialize)]
struct TandCResponse {
    version: String,
    content: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct RegistrationResponse {
    #[serde(rename = "registrationReceipt")]
    registration_receipt: Option<RegistrationReceipt>,
    #[serde(flatten)]
    extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct RegistrationReceipt {
    preimage: String,
    signature: String,
    timestamp: String,
}

#[derive(Debug, Deserialize)]
struct ChallengeResponse {
    code: String,
    challenge: Challenge,
    #[serde(rename = "mining_period_ends")]
    mining_period_ends: String,
}

#[derive(Debug, Deserialize)]
struct Challenge {
    challenge_id: String,
    day: u32,
    challenge_number: u32,
    difficulty: String,
    no_pre_mine: String,
    latest_submission: String,
    no_pre_mine_hour: String,
}

#[derive(Debug, Deserialize)]
struct SolutionResponse {
    crypto_receipt: Option<CryptoReceipt>,
    #[serde(flatten)]
    extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct CryptoReceipt {
    preimage: String,
    timestamp: String,
    signature: String,
}

// ==================== API CLIENT ====================

struct ScavengerAPI {
    client: reqwest::Client,
}

impl ScavengerAPI {
    fn new() -> Result<Self> {
        let mut headers = HeaderMap::new();
        
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
            ),
        );
        
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;
        
        Ok(ScavengerAPI { client })
    }
    
    async fn get_terms(&self) -> Result<TandCResponse> {
        let url = format!("{}/TandC", BASE_URL);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            let body = response.text().await?;
            anyhow::bail!("Failed to fetch T&C: {}", body);
        }
        
        Ok(response.json().await?)
    }

    async fn register(
        &self,
        address: &str,
        signature: &str,
        pubkey: &str,
    ) -> Result<RegistrationResponse> {
        let url = format!(
            "{}/register/{}/{}/{}",
            BASE_URL, address, signature, pubkey
        );
        
        let response = self.client.post(&url).send().await?;
        
        if !response.status().is_success() {
            let body = response.text().await?;
            anyhow::bail!("Registration failed: {}", body);
        }
        
        Ok(response.json().await?)
    }

    async fn get_challenge(&self) -> Result<ChallengeResponse> {
        let url = format!("{}/challenge", BASE_URL);
        let response = self.client.get(&url).send().await?;
        Ok(response.json().await?)
    }

    async fn submit_solution(
        &self,
        address: &str,
        challenge_id: &str,
        nonce: &str,
    ) -> Result<SolutionResponse> {
        let url = format!(
            "{}/solution/{}/{}/{}",
            BASE_URL, address, challenge_id, nonce
        );
        
        let response = self.client.post(&url).send().await?;
        Ok(response.json().await?)
    }

    async fn get_star_rate(&self) -> Result<Vec<u64>> {
        let url = format!("{}/work_to_star_rate", BASE_URL);
        let response = self.client.get(&url).send().await?;
        Ok(response.json().await?)
    }
}

// ==================== MINING LOGIC ====================

struct MiningContext {
    rom: Rom,
    nb_loops: u32,
    nb_instrs: u32,
}

impl MiningContext {
    fn new(no_pre_mine: &str, nb_loops: u32, nb_instrs: u32) -> Self {
        println!("🔧 Initializing AshMaize ROM...");
        println!("   Seed: {}...", &no_pre_mine[..16.min(no_pre_mine.len())]);
        println!("   Loops: {}", nb_loops);
        println!("   Instructions: {}", nb_instrs);
        
        // ROM parameters
        const PRE_SIZE: usize = 16 * 1024 * 1024;        // 16 MB
        const ROM_SIZE: usize = 1024 * 1024 * 1024; // 1 GB
        
        let rom = Rom::new(
            no_pre_mine.as_bytes(),
            RomGenerationType::TwoStep {
                pre_size: PRE_SIZE,
                mixing_numbers: 4,
            },
            ROM_SIZE,
        );
        
        println!("✅ ROM initialized ({} MB)", ROM_SIZE / 1_024 / 1_024);
        
        Self { rom, nb_loops, nb_instrs }
    }
    
    fn hash(&self, preimage: &str) -> [u8; 64] {
        hash(preimage.as_bytes(), &self.rom, self.nb_loops, self.nb_instrs)
    }
}

fn meets_difficulty(hash: &[u8], difficulty: &str) -> bool {
    let diff_bytes = match hex::decode(difficulty) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    
    for i in 0..4.min(diff_bytes.len()) {
        if i >= hash.len() {
            return false;
        }
        if hash[i] < diff_bytes[i] {
            return true;
        }
        if hash[i] > diff_bytes[i] {
            return false;
        }
    }
    true
}

fn build_preimage(
    nonce: &str,
    address: &str,
    challenge: &Challenge,
) -> String {
    format!(
        "{}{}{}{}{}{}{}",
        nonce,
        address,
        challenge.challenge_id,
        challenge.difficulty,
        challenge.no_pre_mine,
        challenge.latest_submission,
        challenge.no_pre_mine_hour
    )
}

fn mine_challenge(
    address: &str,
    challenge: &Challenge,
    max_iterations: u64,
) -> Option<String> {
    println!("\n🔨 Mining started");
    println!("   Challenge ID: {}", challenge.challenge_id);
    println!("   Difficulty: {}", challenge.difficulty);
    println!("   Max iterations: {}", max_iterations);
    
    // Initialize AshMaize
    const NB_LOOPS: u32 = 8;
    const NB_INSTRS: u32 = 256;
    let ctx = MiningContext::new(&challenge.no_pre_mine, NB_LOOPS, NB_INSTRS);
    
    let start = Instant::now();
    let mut last_report = Instant::now();
    
    // Start with random nonce to avoid collisions
    use std::time::{SystemTime, UNIX_EPOCH};
    let random_start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    println!("   Starting nonce: 0x{:016x}", random_start);
    
    for i in 0..max_iterations {
        let nonce = random_start.wrapping_add(i);
        let nonce_hex = format!("{:016x}", nonce);

        // Build preimage
        let preimage = build_preimage(&nonce_hex, address, challenge);
        
        // Hash with AshMaize
        let hash = ctx.hash(&preimage);
        
        // Check difficulty
        if meets_difficulty(&hash, &challenge.difficulty) {
            let elapsed = start.elapsed();
            println!("\n✅ FOUND VALID NONCE!");
            println!("   Nonce: 0x{}", nonce_hex);
            println!("   Nonce (dec): {}", nonce);
            println!("   Hash: {}", hex::encode(&hash[..8]));
            println!("   Time: {:.2}s", elapsed.as_secs_f64());
            println!("   Rate: {:.0} H/s", i as f64 / elapsed.as_secs_f64());
            return Some(nonce_hex);
        }
        
        // Progress report every second
        if last_report.elapsed().as_secs() >= 1 {
            let elapsed = start.elapsed().as_secs_f64();
            let rate = i as f64 / elapsed;
            print!("\r   ⛏️  Iteration: {:>10} | Rate: {:>8.0} H/s | Time: {:>6.1}s", 
                i, rate, elapsed);
            io::stdout().flush().unwrap();
            last_report = Instant::now();
        }
    }
    
    println!("\n❌ No valid nonce found in {} iterations", max_iterations);
    None
}

// ==================== REGISTRATION ====================

async fn interactive_register(
    api: &ScavengerAPI,
    address: &str,
) -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                  📝 REGISTRATION PROCESS                    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    
    println!("\n📄 Fetching Terms & Conditions...");
    let tandc = api.get_terms().await?;
    println!("✅ Got T&C version: {}", tandc.version);
    
    println!("\n📋 Message to sign:");
    println!("────────────────────────────────────────────────────────────────");
    println!("{}", tandc.message);
    println!("────────────────────────────────────────────────────────────────");
    
    println!("\n🔐 How to sign with Cardano wallet:");
    println!("════════════════════════════════════════════════════════════════");
    println!("1. Open your Cardano wallet in browser (Nami/Eternl/Yoroi)");
    println!("2. Open Developer Tools (Press F12)");
    println!("3. Go to Console tab");
    println!("4. Copy and paste this code:\n");
    
    println!("const api = await cardano.nami.enable();");
    println!("const addrs = await api.getUsedAddresses();");
    println!("const msg = \"{}\";", tandc.message.replace("\"", "\\\""));
    println!("const signed = await api.signData(addrs[0], Buffer.from(msg).toString('hex'));");
    println!("console.log('Signature:', signed.signature);");
    println!("console.log('Pubkey:', signed.key);");
    
    println!("\n════════════════════════════════════════════════════════════════");
    println!("5. Copy the outputs and paste below\n");
    
    println!("Enter signature:");
    let mut signature = String::new();
    io::stdin().read_line(&mut signature)?;
    let signature = signature.trim().to_string();
    
    println!("Enter public key:");
    let mut pubkey = String::new();
    io::stdin().read_line(&mut pubkey)?;
    let pubkey = pubkey.trim().to_string();
    
    if pubkey.len() != 64 {
        anyhow::bail!("Invalid pubkey length: {} (expected 64)", pubkey.len());
    }
    
    println!("\n📤 Registering...");
    let result = api.register(address, &signature, &pubkey).await?;
    
    if let Some(receipt) = result.registration_receipt {
        println!("✅ Registration successful!");
        println!("   Timestamp: {}", receipt.timestamp);
    } else {
        println!("✅ Registration completed");
    }
    
    Ok(())
}

// ==================== MAIN ====================
fn wait_for_enter() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║            Press ENTER to exit...                            ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              🌙 SCAVENGER MINER v0.2.0                      ║");
    println!("║           Powered by AshMaize Algorithm                     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let api = ScavengerAPI::new()?;
    
    // TODO: Replace with your Cardano address
    println!("Enter your Cardano address:");
    let mut my_address = String::new();
    io::stdin().read_line(&mut my_address)?;
    let my_address = my_address.trim();
    
    println!("\n📍 Address: {}", my_address);
    
    // Registration (optional)
    println!("\nDo you want to register? (y/n)");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() == "y" {
        match interactive_register(&api, my_address).await {
            Ok(_) => println!("\n✅ Registration successful!"),
            Err(e) => {
                println!("\n⚠️  Registration failed: {}", e);
                println!("   Continuing to mining...");
            }
        }
    } else {
        println!("⏭️  Skipping registration");
    }
    
    // Get challenge
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                  📡 FETCHING CHALLENGE                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    
    let challenge_response = api.get_challenge().await?;
    println!("\n✅ Challenge received:");
    println!("   ID: {}", challenge_response.challenge.challenge_id);
    println!("   Day: {}", challenge_response.challenge.day);
    println!("   Challenge #: {}", challenge_response.challenge.challenge_number);
    println!("   Difficulty: {}", challenge_response.challenge.difficulty);
    println!("   Deadline: {}", challenge_response.mining_period_ends);
    
    // Mining
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                      ⛏️  MINING                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    
    println!("\nHow many hashes to try?");
    println!("  100000     = Quick test (~few minutes)");
    println!("  1000000    = Medium test");
    println!("  100000000  = Serious mining (hours)");
    println!("\nEnter number:");
    
    let mut iterations_input = String::new();
    io::stdin().read_line(&mut iterations_input)?;
    let max_iterations: u64 = iterations_input
        .trim()
        .parse()
        .unwrap_or(100_000);
    
    if let Some(nonce) = mine_challenge(
        my_address,
        &challenge_response.challenge,
        max_iterations,
    ) {
        // Submit solution
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║                  📤 SUBMITTING SOLUTION                      ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        
        let result = api.submit_solution(
            my_address,
            &challenge_response.challenge.challenge_id,
            &nonce,
        ).await?;
        
        if let Some(receipt) = result.crypto_receipt {
            println!("\n🎉🎉🎉 SOLUTION ACCEPTED! 🎉🎉🎉");
            println!("   Timestamp: {}", receipt.timestamp);
            
            // Check reward
            if let Ok(rates) = api.get_star_rate().await {
                let day = challenge_response.challenge.day as usize;
                if day > 0 && day <= rates.len() {
                    println!("\n⭐ REWARD: {} STAR tokens!", rates[day - 1]);
                }
            }
        } else {
            println!("\n📋 Solution submitted");
            println!("   Response: {:?}", result.extra);
        }
    }
    
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    ✅ PROGRAM COMPLETE                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    wait_for_enter();
    Ok(())
}