use ashmaize::{hash, Rom, RomGenerationType};
use hex;

fn main() {
    println!("🧪 Testing AshMaize Integration\n");
    
    // Test 1: Basic hash
    println!("Test 1: Creating ROM...");
    let rom = Rom::new(
        b"test_seed",
        RomGenerationType::FullRandom,
        1024 * 1024,  // 1 MB
    );
    println!("✅ ROM created\n");
    
    // Test 2: Hash computation
    println!("Test 2: Computing hash...");
    let digest = hash(b"hello world", &rom, 8, 256);
    println!("✅ Hash: {}", hex::encode(&digest[..16]));
    
    // Test 3: Scavenger-like parameters
    println!("\nTest 3: Scavenger parameters...");
    let no_pre_mine = "0019c96b6a30ee380019c96b6a30ee38";
    let seed = hex::decode(no_pre_mine).unwrap();
    
    const PRE_SIZE: usize = 16 * 1024;
    const ROM_SIZE: usize = 10 * 1024 * 1024;
    
    let rom2 = Rom::new(
        &seed,
        RomGenerationType::TwoStep {
            pre_size: PRE_SIZE,
            mixing_numbers: 4,
        },
        ROM_SIZE,
    );
    
    let digest2 = hash(b"test", &rom2, 8, 256);
    println!("✅ Hash: {}", hex::encode(&digest2[..8]));
    
    println!("\n✅ All tests passed!");
}