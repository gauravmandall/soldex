use anchor_lang::prelude::*;
use crate::ctx_accounts::UndelegatePosition;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;

pub fn undelegate_position_handler(
    ctx: Context<UndelegatePosition>,
    _market_id: [u8; 16],
    _nonce: u8,
) -> Result<()> {
    // The account is owned by DELeGG on the ER — no Anchor discriminator.
    // We confirmed via byte dump that is_delegated lives at byte 114.
    // Set it to 0 before the CPI so the committed data reflects undelegation.
    {
        let mut data = ctx.accounts.position.try_borrow_mut_data()?;
        require!(data.len() >= 115, anchor_lang::error::ErrorCode::AccountDidNotDeserialize);
        data[114] = 0;
    }

    commit_and_undelegate_accounts(
        &ctx.accounts.owner.to_account_info(),
        vec![&ctx.accounts.position.to_account_info()],
        &ctx.accounts.magic_context.to_account_info(),
        &ctx.accounts.magic_program.to_account_info(),
        None,
    )
    .map_err(|e| anchor_lang::error::Error::from(e))?;

    Ok(())
}
