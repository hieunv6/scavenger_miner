use reqwest;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use std::time::Instant;

// ==================== API RESPONSE STRUCTURES ====================

#[derive(Debug, Deserialize)]
struct TandCResponse {
    version: String,
    content: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct RegistrationResponse {
    #[serde(rename = "registrationReceipt")]
    registration_receipt: RegistrationReceipt,
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

const BASE_URL: &str = "https://scavenger.prod.gd.midnighttge.io";

struct ScavengerAPI {
    client: reqwest::Client,
}

impl ScavengerAPI {
    fn new() -> Self {
        ScavengerAPI {
            client: reqwest::Client::new(),
        }
    }

    // 1. GET Terms & Conditions
    async fn get_terms(&self) -> Result<TandCResponse> {
        let url = format!("{}/TandC", BASE_URL);
        println!("📄 Fetching Terms & Conditions from: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch T&C")?;
        
        println!("   Status: {}", response.status());
        
        let tandc: TandCResponse = response
            .json()
            .await
            .context("Failed to parse T&C response")?;
        
        Ok(tandc)
    }

    // 2. POST Register Address
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
        
        println!("📝 Registering address: {}", address);
        println!("   URL length: {} chars", url.len());
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .context("Failed to register")?;
        
        println!("   Status: {}", response.status());
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            println!("   Error body: {}", error_text);
            anyhow::bail!("Registration failed: {}", error_text);
        }
        
        let result: RegistrationResponse = response
            .json()
            .await
            .context("Failed to parse registration response")?;
        
        Ok(result)
    }

    // 3. GET Challenge
    async fn get_challenge(&self) -> Result<ChallengeResponse> {
        let url = format!("{}/challenge", BASE_URL);
        println!("📡 Fetching challenge from: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch challenge")?;
        
        let challenge: ChallengeResponse = response
            .json()
            .await
            .context("Failed to parse challenge")?;
        
        Ok(challenge)
    }

    // 4. POST Submit Solution
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
        
        println!("📤 Submitting solution...");
        println!("   Challenge: {}", challenge_id);
        println!("   Nonce: {}", nonce);
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .context("Failed to submit solution")?;
        
        println!("   Status: {}", response.status());
        
        let result: SolutionResponse = response
            .json()
            .await
            .context("Failed to parse solution response")?;
        
        Ok(result)
    }

    // 5. GET Work to Star Rate
    async fn get_star_rate(&self) -> Result<Vec<u64>> {
        let url = format!("{}/work_to_star_rate", BASE_URL);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch star rate")?;
        
        let rates: Vec<u64> = response
            .json()
            .await
            .context("Failed to parse star rate")?;
        
        Ok(rates)
    }
}

// ==================== MINING LOGIC ====================

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

// Placeholder mining function
fn mine_challenge(
    address: &str,
    challenge: &Challenge,
    max_iterations: u64,
) -> Option<String> {
    println!("\n🔨 Starting mining...");
    println!("   Challenge ID: {}", challenge.challenge_id);
    println!("   Difficulty: {}", challenge.difficulty);
    println!("   Max iterations: {}", max_iterations);
    
    let start = Instant::now();
    let mut last_report = Instant::now();
    let mut nonce: u64 = 0;
    
    while nonce < max_iterations {
        let nonce_hex = format!("{:016x}", nonce);
        
        // Build preimage
        let preimage = build_preimage(&nonce_hex, address, challenge);
        
        // TODO: Hash with AshMaize
        // let hash = ashmaize_hash(&preimage);
        
        // TODO: Check difficulty
        // if meets_difficulty(&hash, &challenge.difficulty) {
        //     println!("✅ Found valid nonce: {}", nonce_hex);
        //     return Some(nonce_hex);
        // }
        
        // Debug output every second
        if last_report.elapsed().as_secs() >= 1 {
            let elapsed = start.elapsed().as_secs_f64();
            let rate = nonce as f64 / elapsed;
            println!("   ⛏️  Nonce: {} | Rate: {:.0} H/s", nonce, rate);
            last_report = Instant::now();
        }
        
        nonce += 1;
    }
    
    println!("❌ No valid nonce found in {} iterations", max_iterations);
    None
}

// ==================== WALLET FUNCTIONS ====================

// TODO: Implement proper Cardano wallet integration
fn get_cardano_signature(message: &str) -> Result<(String, String)> {
    println!("\n🔐 SIGNATURE REQUIRED");
    println!("════════════════════════════════════════════════════════════════");
    println!("Message to sign:");
    println!("{}", message);
    println!("════════════════════════════════════════════════════════════════");
    println!("\nPlease sign this message with your Cardano wallet.");
    println!("Instructions:");
    println!("1. Open your Cardano wallet (Nami, Eternl, etc.)");
    println!("2. Go to Developer Tools (F12 in browser)");
    println!("3. Run the following in Console:");
    println!("\n--- JavaScript Code ---");
    println!("const api = await cardano.nami.enable();");
    println!("const addresses = await api.getUsedAddresses();");
    println!("const message = `{}`;", message);
    println!("const signed = await api.signData(addresses[0], Buffer.from(message).toString('hex'));");
    println!("console.log('Signature:', signed.signature);");
    println!("console.log('Pubkey:', signed.key);");
    println!("--- End Code ---\n");
    
    println!("Enter signature (CIP-30 format):");
    let mut signature = String::new();
    std::io::stdin().read_line(&mut signature)?;
    let signature = signature.trim().to_string();
    
    println!("Enter public key (64 hex chars):");
    let mut pubkey = String::new();
    std::io::stdin().read_line(&mut pubkey)?;
    let pubkey = pubkey.trim().to_string();
    
    if pubkey.len() != 64 {
        anyhow::bail!("Invalid pubkey length: {} (expected 64)", pubkey.len());
    }
    
    Ok((signature, pubkey))
}

// ==================== MAIN FUNCTION ====================

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           🌙 SCAVENGER MINER v0.1.0                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Initialize API client
    let api = ScavengerAPI::new();
    
    // TODO: Replace with your Cardano address
    let my_address = "addr1qqwzupt3gvehw9s92w2qwsjkk7wl0lpkq5vq9n80cxed80e2wumjk88lz63vlc0f5c6tl2hrca8geqvguczr74ezjhcq2x66y3";
    
    println!("📍 Your address: {}\n", my_address);
    
    // ========== STEP 1: Get Terms & Conditions ==========
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 1: Fetching Terms & Conditions");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let tandc = api.get_terms().await?;
    println!("✅ Got T&C version: {}", tandc.version);
    println!("   Content length: {} chars", tandc.content.len());
    println!("   Message to sign: {}", tandc.message);
    
    // ========== STEP 2: Register Address ==========
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 2: Register Address");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    println!("\nDo you want to register this address? (y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() == "y" {
        // Get signature from user
        let (signature, pubkey) = get_cardano_signature(&tandc.message)?;
        
        println!("\n📝 Attempting registration...");
        match api.register(my_address, &signature, &pubkey).await {
            Ok(reg_response) => {
                println!("✅ Registration successful!");
                println!("   Timestamp: {}", reg_response.registration_receipt.timestamp);
                println!("   Signature: {}...", &reg_response.registration_receipt.signature[..16]);
            }
            Err(e) => {
                println!("⚠️  Registration failed: {}", e);
                println!("   This might be OK if you're already registered.");
                println!("   Continuing to mining...");
            }
        }
    } else {
        println!("⏭️  Skipping registration (assuming already registered)");
    }
    
    // ========== STEP 3: Get Challenge ==========
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 3: Fetching Current Challenge");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let challenge_response = api.get_challenge().await?;
    println!("✅ Got challenge!");
    println!("   Status: {}", challenge_response.code);
    println!("   Challenge ID: {}", challenge_response.challenge.challenge_id);
    println!("   Day: {}", challenge_response.challenge.day);
    println!("   Challenge #: {}", challenge_response.challenge.challenge_number);
    println!("   Difficulty: {}", challenge_response.challenge.difficulty);
    println!("   Deadline: {}", challenge_response.mining_period_ends);
    
    // ========== STEP 4: Mine ==========
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 4: Mining");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let max_iterations = 100_000; // Limit for testing
    
    if let Some(nonce) = mine_challenge(
        my_address,
        &challenge_response.challenge,
        max_iterations,
    ) {
        // ========== STEP 5: Submit Solution ==========
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("STEP 5: Submitting Solution");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        let solution_result = api.submit_solution(
            my_address,
            &challenge_response.challenge.challenge_id,
            &nonce,
        ).await?;
        
        if let Some(receipt) = solution_result.crypto_receipt {
            println!("✅ Solution accepted!");
            println!("   Timestamp: {}", receipt.timestamp);
            println!("   Signature: {}...", &receipt.signature[..16]);
        } else {
            println!("⚠️  Solution submitted but no receipt returned");
            println!("   Response: {:?}", solution_result.extra);
        }
    } else {
        println!("\n⚠️  Mining stopped. No valid nonce found.");
        println!("   Note: AshMaize algorithm not implemented yet.");
        println!("   This is expected with the placeholder code.");
    }
    
    // ========== BONUS: Get Star Rate ==========
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("BONUS: Checking Reward Rates");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    match api.get_star_rate().await {
        Ok(rates) => {
            println!("✅ Star rates (STAR per solution):");
            for (day, rate) in rates.iter().enumerate() {
                println!("   Day {:2}: {:>10} STAR", day + 1, rate);
            }
        }
        Err(e) => {
            println!("⚠️  Failed to fetch star rates: {}", e);
        }
    }
    
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    🎉 PROGRAM COMPLETE                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    
    Ok(())
}