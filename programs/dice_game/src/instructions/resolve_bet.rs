use anchor_lang::{prelude::*, solana_program::sysvar::instructions::load_instruction_at_checked, system_program::{Transfer,transfer}};
use anchor_instruction_sysvar::Ed25519InstructionSignatures;
use solana_program::hash::hash;
use solana_program::ed25519_program;

use crate::state::Bet;

#[derive(Accounts)]
#[instruction(seed: u128)]
pub struct ResolveBet<'info> {
    #[account(mut)]
    pub house: Signer<'info>,
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", house.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        seeds = [b"bet", vault.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump
    )]
    pub bet: Account<'info, Bet>,
    /// CHECK: This is the instruction sysvar, safe to use as it's read-only and provided by the runtime
    #[account(
        address = solana_program::sysvar::instructions::ID,
    )]
    pub instruction_sysvar: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> ResolveBet<'info> {
    pub fn verify_ed25519_signature(&mut self, sig: &[u8]) -> Result<()> {
        let ix= load_instruction_at_checked(0, &self.instruction_sysvar)?;
        require_keys_eq!(ix.program_id, ed25519_program::ID);
        require_eq!(ix.accounts.len(), 0);

        let signatures = Ed25519InstructionSignatures::unpack(sig)?.0;
        require_eq!(signatures.len(), 1);
        let signature = &signatures[0];
        require_eq!(signature.is_verifiable, true);
        require_keys_eq!(signature.public_key.unwrap(), self.house.key());
        require_eq!(signature.signature.unwrap().eq(sig), true);

        require_eq!(signature.message.as_ref().unwrap().eq(&self.bet.to_slice()), true);
        Ok(())
    }
    
    
    pub fn resolve_bet(&mut self, sig: &[u8], bumps: &ResolveBetBumps) -> Result<()> {

        let hash = hash(sig).to_bytes();

        let mut hash_16 = [0u8; 16];

        hash_16.copy_from_slice(&hash[0..16]);
        let lower = u128::from_le_bytes(hash_16);

        hash_16.copy_from_slice(&hash[16..32]);
        let upper = u128::from_le_bytes(hash_16);

        let roll = lower.wrapping_add(upper).wrapping_rem(100) as u8 +1;

        if self.bet.roll > roll {
            let payout = (self.bet.amount as u128)
            .checked_mul(10000 - 150 as u128).unwrap()
            .checked_div(self.bet.roll as u128).unwrap()
            .checked_div(10000).unwrap();
        let cpi_program = self.system_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.player.to_account_info()
        };
        let seeds = [b"vault", &self.house.key().to_bytes()[..], &[bumps.vault]];
        let signer_seeds = &[&seeds[..][..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer(cpi_ctx, payout as u64)?;

        }
        Ok(())
    }
}