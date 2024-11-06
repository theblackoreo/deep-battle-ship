use anchor_lang::{
    prelude::*,
    system_program::{create_account, CreateAccount},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};

use spl_transfer_hook_interface::instruction::{ExecuteInstruction, TransferHookInstruction};
use anchor_lang::solana_program::clock::Clock;


declare_id!("3vkhRPB5twHMGh84Fo9kyEwFyYBaYoNTVy4oQa3HdSSf");

#[program]
mod battleship_game {
    use super::*;

    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {

        // The `addExtraAccountsToInstruction` JS helper function resolving incorrectly
        let account_metas = vec![
        ExtraAccountMeta::new_with_seeds(
            &[Seed::Literal {
                bytes: "counter".as_bytes().to_vec(),
            }],
            false, // is_signer
            true,  // is_writable
        )?,
    ];

        // calculate account size
        let account_size = ExtraAccountMetaList::size_of(account_metas.len())? as u64;
        // calculate minimum required lamports
        let lamports = Rent::get()?.minimum_balance(account_size as usize);

        let mint = ctx.accounts.mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"extra-account-metas",
            &mint.as_ref(),
            &[ctx.bumps.extra_account_meta_list],
        ]];

        // create ExtraAccountMetaList account
        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.extra_account_meta_list.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            lamports,
            account_size,
            ctx.program_id,
        )?;

        

        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &account_metas,
        )?;
        
        Ok(())
    }
    

    

     //Hook
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        //assert_is_transferring(&ctx)?;

        // msg!("ddd {}", ctx.accounts.game_state.game_id.winner); 
        //initilization
        if(ctx.accounts.game_state.game_id.winner == Some(0)){ 
            initialize(ctx);
           
            }else{

        msg!(&format!("Transfer hook fired for an amount of {}", amount));

        let amout_to_pass: u64 = amount as u64;
        
        make_move(ctx, amout_to_pass);
         
            }

        Ok(())
    }

    pub fn initialize(ctx: Context<TransferHook>) -> Result<()> {

         msg!("Initializing all the game...");

        let game_state = &mut ctx.accounts.game_state;

        game_state.player1_ships = generate_random_ships(1); 
        game_state.player2_ships = generate_random_ships(2);
        game_state.turn = 1; 
        game_state.targeting_ship = None;
        game_state.game_id.id = 0;
        
        game_state.game_id.time_started = Some(Clock::get()?.unix_timestamp as u64);
        
        game_state.game_id.winner = None;

        Ok(())
    }


    pub fn make_move(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let current_turn = game_state.turn;

        let target = if let Some(targeting_ship) = game_state.targeting_ship {
            get_next_target(targeting_ship) 
        } else {
            Position {
                x: (amount % 20) as u8,
                y: ((amount / 20) % 20) as u8,
            }
        };

        let hit = if current_turn == 1 {
            game_state.player2_ships.iter().any(|ship| ship_occupies(ship, target))
        } else {
            game_state.player1_ships.iter().any(|ship| ship_occupies(ship, target))
        };

          // Print player ship positions before the move
        msg!("Player 1 Ships: {:?}", game_state.player1_ships);
        msg!("Player 2 Ships: {:?}", game_state.player2_ships);

        msg!("Player {} targeting position ({}, {}) - Hit: {}", current_turn, target.x, target.y, hit);

        if hit {
            game_state.targeting_ship = Some(target);

            if current_turn == 1 {
                game_state.player2_ships.retain(|ship| !ship_occupies(ship, target));
                if game_state.player2_ships.is_empty() {
                    game_state.game_id.winner = Some(1);
                    
                    msg!("GameID, TimeBegin, TimeEnded, Winner");
                    msg!("{}, {:?}, {}, {:?}", game_state.game_id.id, game_state.game_id.time_started ,1, Some(Clock::get()?.unix_timestamp as u64));
                    reset_game(game_state);
                    game_state.game_id.time_started =  Some(Clock::get()?.unix_timestamp as u64);
                    
                    return Ok(());
                }
            } else {
                game_state.player1_ships.retain(|ship| !ship_occupies(ship, target));
                if game_state.player1_ships.is_empty() {
                    game_state.game_id.winner = Some(2);
                    msg!("GameID, TimeBegin, TimeEnded, Winner");
                    msg!("{}, {:?}, {}, {:?}", game_state.game_id.id, game_state.game_id.time_started ,2,  Some(Clock::get()?.unix_timestamp as u64));
                    reset_game(game_state);
                    game_state.game_id.time_started = Some(Clock::get()?.unix_timestamp as u64);
                    return Ok(());
                }
            }
        } else {
            game_state.targeting_ship = None; 
        }

        game_state.turn = if game_state.turn == 1 { 2 } else { 1 };
        Ok(())
    }

    pub fn fallback<'info>(
        program_id: &Pubkey,
        accounts: &'info [AccountInfo<'info>],
        data: &[u8],
    ) -> Result<()> {
        let instruction = TransferHookInstruction::unpack(data)?;

        // match instruction discriminator to transfer hook interface execute instruction
        // token2022 program CPIs this instruction on token transfer
        match instruction {
            TransferHookInstruction::Execute { amount } => {
                let amount_bytes = amount.to_le_bytes();

                // invoke custom transfer hook instruction on our program
                __private::__global::transfer_hook(program_id, accounts, &amount_bytes)
            }
            _ => return Err(ProgramError::InvalidInstructionData.into()),
        }
    }
    
}

fn reset_game(game_state: &mut GameState) {
    game_state.player1_ships = generate_random_ships(1);
    game_state.player2_ships = generate_random_ships(2);
    game_state.turn = 1;
    game_state.targeting_ship = None;
    
    
}

fn generate_random_ships(seed: u8) -> Vec<Ship> {
    let ship_lengths = [2, 3, 3, 4, 5];
    let mut ships = Vec::new();
    let mut x = seed;
    let mut y = seed * 2;

    for &length in ship_lengths.iter() {
        let mut placed = false;

        while !placed {
            let direction = if (x + y) % 2 == 0 { Direction::Horizontal } else { Direction::Vertical };
            let start_position = Position { x: (x + length) % 20, y: (y + length) % 20 };

            let (end_x, end_y) = match direction {
                Direction::Horizontal => (start_position.x + length - 1, start_position.y),
                Direction::Vertical => (start_position.x, start_position.y + length - 1),
            };

            if end_x < 20 && end_y < 20 && !ship_overlaps(&ships, start_position, direction, length) {
                ships.push(Ship {
                    start_position,
                    direction,
                    length,
                });
                placed = true;
            }

            x = (x + 3) % 20;
            y = (y + 7) % 20;
        }
    }
    ships
}

fn ship_overlaps(ships: &Vec<Ship>, start: Position, direction: Direction, length: u8) -> bool {
    for ship in ships.iter() {
        for i in 0..length {
            let pos = match direction {
                Direction::Horizontal => Position { x: start.x + i, y: start.y },
                Direction::Vertical => Position { x: start.x, y: start.y + i },
            };

            for j in 0..ship.length {
                let ship_pos = match ship.direction {
                    Direction::Horizontal => Position { x: ship.start_position.x + j, y: ship.start_position.y },
                    Direction::Vertical => Position { x: ship.start_position.x, y: ship.start_position.y + j },
                };

                if pos == ship_pos {
                    return true;
                }
            }
        }
    }
    false
}

fn ship_occupies(ship: &Ship, position: Position) -> bool {
    for i in 0..ship.length {
        let ship_pos = match ship.direction {
            Direction::Horizontal => Position { x: ship.start_position.x + i, y: ship.start_position.y },
            Direction::Vertical => Position { x: ship.start_position.x, y: ship.start_position.y + i },
        };

        if ship_pos == position {
            return true;
        }
    }
    false
}

fn get_next_target(last_hit: Position) -> Position {
    if last_hit.x < 19 {
        Position { x: last_hit.x + 1, y: last_hit.y }
    } else if last_hit.x > 0 {
        Position { x: last_hit.x - 1, y: last_hit.y }
    } else if last_hit.y < 19 {
        Position { x: last_hit.x, y: last_hit.y + 1 }
    } else {
        Position { x: last_hit.x, y: last_hit.y - 1 }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub x: u8,
    pub y: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct Ship {
    pub start_position: Position,
    pub direction: Direction,
    pub length: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct GameID {
    pub id: u64,
    pub time_started: Option<u64>,
    pub winner: Option<u8>,
}



#[account]
pub struct GameState {
    pub player1_ships: Vec<Ship>,
    pub player2_ships: Vec<Ship>,
    pub turn: u8,
    pub targeting_ship: Option<Position>,
    pub game_id: GameID,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 145)]
    pub game_state: Account<'info, GameState>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MakeMove<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct TransferHook<'info> {
  #[account(token::mint = mint, token::authority = owner)]
  pub source_token: InterfaceAccount<'info, TokenAccount>,
  pub mint: InterfaceAccount<'info, Mint>,
  #[account(token::mint = mint)]
  pub destination_token: InterfaceAccount<'info, TokenAccount>,
  /// CHECK: source token account owner, 
  /// can be SystemAccount or PDA owned by another program
  pub owner: UncheckedAccount<'info>,
  #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,

    #[account(
            mut,
            seeds = [b"counter"],
            bump
        )]
    pub game_state: Account<'info, GameState>,
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        mut,
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        seeds = [b"counter"], 
        bump,
        payer = payer,
        space = 8 + 145
    )]
    pub game_state: Account<'info, GameState>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}


#[error_code]
pub enum ErrorCode {
    #[msg("There was an error calculating the time since UNIX epoch.")]
    TimeCalculationError,
    // Aggiungi altri errori personalizzati qui
    #[msg("An invalid move was attempted.")]
    InvalidMove,
}