#![no_std]
#![allow(unexpected_cfgs)]

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{find_program_address, Pubkey},
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::TransferChecked;

entrypoint!(process_instruction);
pinocchio::nostd_panic_handler!();

pub const ID: Pubkey = pinocchio_pubkey::pubkey!("4cJDBQmPnuf3GZrP9SW17wDhPpBVcvfNk51MiMrUCCfQ");
pub const ADMIN: Pubkey = pinocchio_pubkey::pubkey!("Dy6mBH4YeqJCRZohd39iSFaf4jyLaxPeBakbZwt1jToL");
pub const PYTH_RECEIVER: Pubkey =
    pinocchio_pubkey::pubkey!("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ");
pub const TOKEN_PROGRAM: Pubkey =
    pinocchio_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

const CONFIG_LEN: usize = 72;
const RESERVE_LEN: usize = 224;
const POSITION_LEN: usize = 128;

#[repr(u32)]
enum Error {
    InvalidInstruction = 1,
    InvalidAccounts,
    MissingSignature,
    InvalidOwner,
    InvalidPda,
    AlreadyInitialized,
    Unauthorized,
    Paused,
    InvalidAmount,
    InvalidTokenAccount,
    InvalidOracle,
    StalePrice,
    PriceConfidence,
    InsufficientLiquidity,
    HealthFactor,
    Overflow,
}

fn err(e: Error) -> ProgramError {
    ProgramError::Custom(e as u32)
}
fn read_u16(d: &[u8], o: usize) -> Result<u16, ProgramError> {
    Ok(u16::from_le_bytes(
        d.get(o..o + 2)
            .ok_or(err(Error::InvalidInstruction))?
            .try_into()
            .unwrap(),
    ))
}
fn read_u64(d: &[u8], o: usize) -> Result<u64, ProgramError> {
    Ok(u64::from_le_bytes(
        d.get(o..o + 8)
            .ok_or(err(Error::InvalidInstruction))?
            .try_into()
            .unwrap(),
    ))
}
fn write_u64(d: &mut [u8], o: usize, v: u64) {
    d[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn key_at(d: &[u8], o: usize) -> Pubkey {
    d[o..o + 32].try_into().unwrap()
}
fn require(v: bool, e: Error) -> ProgramResult {
    if v {
        Ok(())
    } else {
        Err(err(e))
    }
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    require(program_id == &ID, Error::InvalidOwner)?;
    match data
        .first()
        .copied()
        .ok_or(err(Error::InvalidInstruction))?
    {
        0 => initialize_config(accounts),
        1 => initialize_reserve(accounts, data),
        2 => set_reserve(accounts, data),
        3 => initialize_position(accounts),
        4 => deposit(accounts, data),
        5 => withdraw(accounts, data),
        6 => borrow(accounts, data),
        7 => repay(accounts, data),
        8 => liquidate(accounts, data),
        9 => set_pause(accounts, data),
        _ => Err(err(Error::InvalidInstruction)),
    }
}

fn create_pda<'a>(
    payer: &'a AccountInfo,
    account: &'a AccountInfo,
    seeds: &[Seed],
    bump: u8,
    space: usize,
) -> ProgramResult {
    require(payer.is_signer(), Error::MissingSignature)?;
    require(
        account.lamports() == 0 && account.data_len() == 0,
        Error::AlreadyInitialized,
    )?;
    let bump_seed = [bump];
    let mut all = [Seed::from(b""), Seed::from(b""), Seed::from(&bump_seed)];
    for (i, seed) in seeds.iter().enumerate() {
        all[i] = seed.clone();
    }
    let signer = Signer::from(&all[..seeds.len() + 1]);
    CreateAccount {
        from: payer,
        to: account,
        lamports: Rent::get()?.minimum_balance(space),
        space: space as u64,
        owner: &ID,
    }
    .invoke_signed(&[signer])
}

fn initialize_config(a: &[AccountInfo]) -> ProgramResult {
    require(a.len() == 2, Error::InvalidAccounts)?;
    require(
        a[0].is_signer() && a[0].key() == &ADMIN,
        Error::Unauthorized,
    )?;
    let (pda, bump) = find_program_address(&[b"config"], &ID);
    require(a[1].key() == &pda, Error::InvalidPda)?;
    create_pda(&a[0], &a[1], &[Seed::from(b"config")], bump, CONFIG_LEN)?;
    let mut d = a[1].try_borrow_mut_data()?;
    d[0] = 1;
    d[1] = bump;
    d[2] = 0;
    d[8..40].copy_from_slice(&ADMIN);
    Ok(())
}

// data: tag | mint(32) | feed_id(32) | decimals u8 | ltv bps u16 | liquidation threshold bps u16 |
// liquidation bonus bps u16 | reserve factor bps u16 | supply cap u64 | borrow cap u64 | max age u64 | max conf bps u16
fn initialize_reserve(a: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    require(a.len() == 3 && ix.len() == 100, Error::InvalidAccounts)?;
    assert_admin(&a[0], &a[1])?;
    let mint: Pubkey = ix[1..33].try_into().unwrap();
    require(
        a[2].key() == &find_program_address(&[b"reserve", mint.as_ref()], &ID).0,
        Error::InvalidPda,
    )?;
    let bump = find_program_address(&[b"reserve", mint.as_ref()], &ID).1;
    create_pda(
        &a[0],
        &a[2],
        &[Seed::from(b"reserve"), Seed::from(mint.as_ref())],
        bump,
        RESERVE_LEN,
    )?;
    let mut d = a[2].try_borrow_mut_data()?;
    d[0] = 1;
    d[1] = bump;
    d[2] = ix[65];
    d[3] = 0;
    d[8..40].copy_from_slice(&mint);
    d[40..72].copy_from_slice(&ix[33..65]);
    d[72..80].copy_from_slice(&ix[66..74]);
    d[80..88].copy_from_slice(&ix[74..82]);
    d[88..96].copy_from_slice(&ix[82..90]);
    d[96..104].copy_from_slice(&ix[90..98]);
    d[160..162].copy_from_slice(&ix[98..100]);
    write_u64(&mut d, 104, 1_000_000_000);
    write_u64(&mut d, 112, 1_000_000_000);
    write_u64(&mut d, 152, Clock::get()?.unix_timestamp.max(0) as u64);
    Ok(())
}

// data: tag | ltv | threshold | bonus | factor | supply cap | borrow cap | max age | max conf
fn set_reserve(a: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    require(a.len() == 3 && ix.len() == 35, Error::InvalidAccounts)?;
    assert_admin(&a[0], &a[1])?;
    require(a[2].owner() == &ID, Error::InvalidOwner)?;
    let l = read_u16(ix, 1)?;
    let t = read_u16(ix, 3)?;
    require(
        l <= t && t <= 10_000 && read_u16(ix, 7)? <= 10_000,
        Error::InvalidAmount,
    )?;
    let mut d = a[2].try_borrow_mut_data()?;
    d[72..80].copy_from_slice(&ix[1..9]);
    d[80..104].copy_from_slice(&ix[9..33]);
    d[160..162].copy_from_slice(&ix[33..35]);
    Ok(())
}

fn initialize_position(a: &[AccountInfo]) -> ProgramResult {
    require(a.len() == 2 && a[0].is_signer(), Error::InvalidAccounts)?;
    let (p, b) = find_program_address(&[b"position", a[0].key().as_ref()], &ID);
    require(a[1].key() == &p, Error::InvalidPda)?;
    create_pda(
        &a[0],
        &a[1],
        &[Seed::from(b"position"), Seed::from(a[0].key().as_ref())],
        b,
        POSITION_LEN,
    )?;
    let mut d = a[1].try_borrow_mut_data()?;
    d[0] = 1;
    d[1] = b;
    d[8..40].copy_from_slice(a[0].key());
    Ok(())
}

fn assert_admin(admin: &AccountInfo, config: &AccountInfo) -> ProgramResult {
    require(admin.is_signer(), Error::MissingSignature)?;
    require(
        config.owner() == &ID && config.data_len() == CONFIG_LEN,
        Error::InvalidOwner,
    )?;
    let d = config.try_borrow_data()?;
    require(admin.key() == &key_at(&d, 8), Error::Unauthorized)
}
fn validate_common(a: &[AccountInfo]) -> Result<(&AccountInfo, &AccountInfo), ProgramError> {
    require(a.len() == 8, Error::InvalidAccounts)?;
    require(a[0].is_signer(), Error::MissingSignature)?;
    require(
        a[1].owner() == &ID && a[2].owner() == &ID,
        Error::InvalidOwner,
    )?;
    let c = a[1].try_borrow_data()?;
    require(c[3] == 0, Error::Paused)?;
    drop(c);
    let p = a[2].try_borrow_data()?;
    require(&key_at(&p, 8) == a[0].key(), Error::Unauthorized)?;
    drop(p);
    let r = a[1].try_borrow_data()?;
    require(&key_at(&r, 8) == a[4].key(), Error::InvalidTokenAccount)?;
    drop(r);
    require(a[7].key() == &TOKEN_PROGRAM, Error::InvalidTokenAccount)?;
    Ok((&a[1], &a[2]))
}
fn token_amount(a: &AccountInfo) -> Result<u64, ProgramError> {
    require(
        a.owner() == &TOKEN_PROGRAM && a.data_len() >= 72,
        Error::InvalidTokenAccount,
    )?;
    let d = a.try_borrow_data()?;
    Ok(u64::from_le_bytes(d[64..72].try_into().unwrap()))
}
fn token_owner(a: &AccountInfo) -> Result<Pubkey, ProgramError> {
    let d = a.try_borrow_data()?;
    require(d.len() >= 64, Error::InvalidTokenAccount)?;
    Ok(key_at(&d, 32))
}
fn oracle_price(reserve: &[u8], oracle: &AccountInfo) -> Result<u128, ProgramError> {
    require(oracle.owner() == &PYTH_RECEIVER, Error::InvalidOracle)?;
    let d = oracle.try_borrow_data()?;
    require(d.len() >= 126 && d[40] == 0, Error::InvalidOracle)?;
    let off = 41;
    require(d[off..off + 32] == reserve[40..72], Error::InvalidOracle)?;
    let price = i64::from_le_bytes(d[off + 32..off + 40].try_into().unwrap());
    let conf = u64::from_le_bytes(d[off + 40..off + 48].try_into().unwrap());
    let expo = i32::from_le_bytes(d[off + 48..off + 52].try_into().unwrap());
    let publish = i64::from_le_bytes(d[off + 60..off + 68].try_into().unwrap());
    require(price > 0 && (-18..=0).contains(&expo), Error::InvalidOracle)?;
    let now = Clock::get()?.unix_timestamp;
    require(
        now >= publish && (now - publish) as u64 <= read_u64(reserve, 96)?,
        Error::StalePrice,
    )?;
    require(
        (conf as u128) * 10_000 <= (price as u128) * (read_u16(reserve, 160)? as u128),
        Error::PriceConfidence,
    )?;
    Ok((price as u128) * 10u128.pow((18 + expo) as u32))
}
fn shares(amount: u64, index: u64) -> Result<u64, ProgramError> {
    u64::try_from((amount as u128) * 1_000_000_000u128 / (index as u128))
        .map_err(|_| err(Error::Overflow))
}
fn amount(shares: u64, index: u64) -> Result<u64, ProgramError> {
    u64::try_from((shares as u128) * (index as u128) / 1_000_000_000u128)
        .map_err(|_| err(Error::Overflow))
}
fn position_values(pos: &[u8], reserve: &[u8], price: u128) -> Result<(u128, u128), ProgramError> {
    let scale = 10u128.pow(reserve[2] as u32);
    let dep = amount(read_u64(pos, 40)?, read_u64(reserve, 104)?)? as u128;
    let debt = amount(read_u64(pos, 48)?, read_u64(reserve, 112)?)? as u128;
    Ok((dep * price / scale, debt * price / scale))
}
fn transfer(
    from: &AccountInfo,
    mint: &AccountInfo,
    to: &AccountInfo,
    auth: &AccountInfo,
    amt: u64,
    dec: u8,
    signer: Option<Signer>,
) -> ProgramResult {
    let t = TransferChecked {
        from,
        mint,
        to,
        authority: auth,
        amount: amt,
        decimals: dec,
    };
    if let Some(s) = signer {
        t.invoke_signed(&[s])
    } else {
        t.invoke()
    }
}

// accounts user, reserve, position, user token, mint, vault, oracle, token program
fn deposit(a: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    let (r, p) = validate_common(a)?;
    let amt = read_u64(ix, 1)?;
    require(ix.len() == 9 && amt > 0, Error::InvalidAmount)?;
    require(
        token_owner(&a[3])? == *a[0].key(),
        Error::InvalidTokenAccount,
    )?;
    let mut rd = r.try_borrow_mut_data()?;
    oracle_price(&rd, &a[6])?;
    let cap = read_u64(&rd, 80)?;
    require(
        read_u64(&rd, 120)?
            .checked_add(amt)
            .ok_or(err(Error::Overflow))?
            <= cap,
        Error::InvalidAmount,
    )?;
    transfer(&a[3], &a[4], &a[5], &a[0], amt, rd[2], None)?;
    let sh = shares(amt, read_u64(&rd, 104)?)?;
    let new_supply = read_u64(&rd, 120)?
        .checked_add(amt)
        .ok_or(err(Error::Overflow))?;
    write_u64(&mut rd, 120, new_supply);
    drop(rd);
    let mut pd = p.try_borrow_mut_data()?;
    let new_deposit = read_u64(&pd, 40)?
        .checked_add(sh)
        .ok_or(err(Error::Overflow))?;
    write_u64(&mut pd, 40, new_deposit);
    Ok(())
}
fn withdraw(a: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    let (r, p) = validate_common(a)?;
    let amt = read_u64(ix, 1)?;
    require(ix.len() == 9 && amt > 0, Error::InvalidAmount)?;
    let mut rd = r.try_borrow_mut_data()?;
    let price = oracle_price(&rd, &a[6])?;
    let mut pd = p.try_borrow_mut_data()?;
    let sh = shares(amt, read_u64(&rd, 104)?)?;
    require(
        read_u64(&pd, 40)? >= sh && token_amount(&a[5])? >= amt,
        Error::InsufficientLiquidity,
    )?;
    let new_deposit = read_u64(&pd, 40)? - sh;
    write_u64(&mut pd, 40, new_deposit);
    let (v, d) = position_values(&pd, &rd, price)?;
    require(
        d == 0 || v * (read_u16(&rd, 74)? as u128) / 10_000 >= d,
        Error::HealthFactor,
    )?;
    let bump = [rd[1]];
    let signer_seeds = [
        Seed::from(b"reserve"),
        Seed::from(a[4].key().as_ref()),
        Seed::from(&bump),
    ];
    let s = Signer::from(&signer_seeds);
    transfer(&a[5], &a[4], &a[3], r, amt, rd[2], Some(s))?;
    let new_supply = read_u64(&rd, 120)? - amt;
    write_u64(&mut rd, 120, new_supply);
    Ok(())
}
fn borrow(a: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    let (r, p) = validate_common(a)?;
    let amt = read_u64(ix, 1)?;
    require(ix.len() == 9 && amt > 0, Error::InvalidAmount)?;
    let mut rd = r.try_borrow_mut_data()?;
    let price = oracle_price(&rd, &a[6])?;
    require(
        token_amount(&a[5])? >= amt
            && read_u64(&rd, 128)?
                .checked_add(amt)
                .ok_or(err(Error::Overflow))?
                <= read_u64(&rd, 88)?,
        Error::InsufficientLiquidity,
    )?;
    let mut pd = p.try_borrow_mut_data()?;
    let sh = shares(amt, read_u64(&rd, 112)?)?;
    let new_debt_shares = read_u64(&pd, 48)?
        .checked_add(sh)
        .ok_or(err(Error::Overflow))?;
    write_u64(&mut pd, 48, new_debt_shares);
    let (v, d) = position_values(&pd, &rd, price)?;
    require(
        v * (read_u16(&rd, 72)? as u128) / 10_000 >= d,
        Error::HealthFactor,
    )?;
    let bump = [rd[1]];
    let signer_seeds = [
        Seed::from(b"reserve"),
        Seed::from(a[4].key().as_ref()),
        Seed::from(&bump),
    ];
    let s = Signer::from(&signer_seeds);
    transfer(&a[5], &a[4], &a[3], r, amt, rd[2], Some(s))?;
    let new_borrows = read_u64(&rd, 128)?
        .checked_add(amt)
        .ok_or(err(Error::Overflow))?;
    write_u64(&mut rd, 128, new_borrows);
    Ok(())
}
fn repay(a: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    let (r, p) = validate_common(a)?;
    let amt = read_u64(ix, 1)?;
    require(ix.len() == 9 && amt > 0, Error::InvalidAmount)?;
    require(
        token_owner(&a[3])? == *a[0].key(),
        Error::InvalidTokenAccount,
    )?;
    let mut rd = r.try_borrow_mut_data()?;
    let mut pd = p.try_borrow_mut_data()?;
    let actual = core::cmp::min(amt, amount(read_u64(&pd, 48)?, read_u64(&rd, 112)?)?);
    transfer(&a[3], &a[4], &a[5], &a[0], actual, rd[2], None)?;
    let sh = shares(actual, read_u64(&rd, 112)?)?;
    let new_debt_shares = read_u64(&pd, 48)?.saturating_sub(sh);
    let new_borrows = read_u64(&rd, 128)?.saturating_sub(actual);
    write_u64(&mut pd, 48, new_debt_shares);
    write_u64(&mut rd, 128, new_borrows);
    Ok(())
}
fn liquidate(_a: &[AccountInfo], _ix: &[u8]) -> ProgramResult {
    Err(err(Error::InvalidInstruction))
}
fn set_pause(a: &[AccountInfo], ix: &[u8]) -> ProgramResult {
    require(a.len() == 3 && ix.len() == 2, Error::InvalidAccounts)?;
    assert_admin(&a[0], &a[1])?;
    require(
        a[2].owner() == &ID && a[2].data_len() == RESERVE_LEN,
        Error::InvalidOwner,
    )?;
    let mut d = a[2].try_borrow_mut_data()?;
    d[3] = u8::from(ix[1] != 0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_math_round_trips_at_initial_index() {
        assert_eq!(shares(42_000, 1_000_000_000).unwrap(), 42_000);
        assert_eq!(amount(42_000, 1_000_000_000).unwrap(), 42_000);
    }

    #[test]
    fn share_math_accounts_for_index_growth() {
        assert_eq!(amount(1_000, 1_100_000_000).unwrap(), 1_100);
    }

    #[test]
    fn malformed_instruction_is_rejected() {
        assert_eq!(
            process_instruction(&ID, &[], &[]),
            Err(ProgramError::Custom(1))
        );
    }

    #[test]
    fn wrong_program_is_rejected() {
        assert_eq!(
            process_instruction(&[0; 32], &[], &[0]),
            Err(ProgramError::Custom(4))
        );
    }
}
