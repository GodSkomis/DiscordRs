use serenity::all::PremiumTier;


pub fn get_bitrate(premium_tier: &PremiumTier) -> u32 {
    match premium_tier {
        PremiumTier::Tier0 => 96000,
        PremiumTier::Tier1 => 128000,
        PremiumTier::Tier2 => 256000,
        PremiumTier::Tier3 => 384000,
        _ => 96000
    }
}