use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Mint, SetAuthority, TokenAccount, Transfer, Token};
use pyth_client::{
    Product,
    Price,
    PriceType,
    PriceInfo,
    load_mapping,
    load_product,
    load_price
};

use solana_program::{
  pubkey::Pubkey
};

pub mod error;

// account address where the program is deployed - DevNet and LocalNet
declare_id!("79265P9aqEzrecFFAucDAtftvw4qDDQbz9rxWruRJRoR");
const MINIMUM_HOLDOUT_SEC: u64 = 5;
const BID_AMOUNT_EXPONENT: u64 = 1000000000;

#[program]
pub mod truesight_protocol {
    use super::*;
    use pyth_client;

    pub fn create_prediction(ctx: Context<CreatePrediction>, direction: String, holdout_period_sec: u64, bid_amount: u64) -> Result<()> {

        // When holdout period is not long enough
        if holdout_period_sec < MINIMUM_HOLDOUT_SEC {
             return Err(error!(ErrorCode::InsufficientHoldOut));

        // When there are not enough TSD coins in wallet
        } else if !ctx.accounts.has_enough_funds(bid_amount) {
             return Err(error!(ErrorCode::InsufficientTSD));

        } else if holdout_period_sec >= MINIMUM_HOLDOUT_SEC && ctx.accounts.has_enough_funds(bid_amount) {

            let prediction_record   = &mut ctx.accounts.prediction_record;

            // Fetch product information from Pyth.Network
            let pyth_product                = &ctx.accounts.asset_record;
            let pyth_product_data           = &pyth_product.try_borrow_data()?;
            let product_account: Product    = *load_product(pyth_product_data).unwrap();
            
            // Fetch price information from Pyth.Network
            let pyth_price_info = &ctx.accounts.asset_price_record;
            let pyth_price_data = &pyth_price_info.try_borrow_data()?;
            let price_account: Price = *load_price(pyth_price_data).unwrap();
            
            for (key, val) in product_account.iter() {
                if key == "symbol" {
                    prediction_record.asset = val.to_string();
                }
            }

            prediction_record.direction                 = direction;
            prediction_record.expiry_date               = (holdout_period_sec as i64) + Clock::get().unwrap().unix_timestamp;
            
            prediction_record.bidder_token_wallet_key   = ctx.accounts.user_token_wallet.key().to_string();
            prediction_record.pyth_product_public_key   = pyth_product.key.to_string();
            prediction_record.pyth_price_public_key     = pyth_price_info.key.to_string();
            prediction_record.entry_price               = price_account.agg.price;
            prediction_record.entry_expo                = price_account.expo;
            prediction_record.bid_amount                = bid_amount;

            // Transfers TSD tokens to our DAO's betting pool
            ctx.accounts.submit_bid(bid_amount);

            // TODO: Transfer ownership of Prediction Record to Betting Pool

        }

        Ok(())
    }

    pub fn validate_prediction(ctx: Context<ValidatePrediction>) -> Result<()> {
        let prediction_record = &mut ctx.accounts.prediction_record;

        // Fetch price information from Pyth.Network
        let pyth_price_info = &ctx.accounts.asset_price_record;
        let pyth_price_data = &pyth_price_info.try_borrow_data()?;
        let price_account: Price = *load_price(pyth_price_data).unwrap();                

        if prediction_record.asset != "" && 
            Clock::get().unwrap().unix_timestamp > prediction_record.expiry_date && 
            prediction_record.pyth_price_public_key == pyth_price_info.key.to_string() {

                prediction_record.validation_date   = Clock::get().unwrap().unix_timestamp;
                prediction_record.validation_price  = price_account.agg.price;
                prediction_record.validation_expo   = price_account.expo;

                let entry_price          = prediction_record.entry_price as f64 * f64::powf(10.0, prediction_record.entry_expo as f64);
                let validation_price     = prediction_record.validation_price as f64 * f64::powf(10.0, prediction_record.validation_expo as f64);

                prediction_record.is_correct = false;

                if prediction_record.direction == "UP" &&  
                    entry_price < validation_price {
                        prediction_record.is_correct = true;

                } else if prediction_record.direction == "DOWN" &&  
                    entry_price > validation_price {
                        prediction_record.is_correct = true;

                }

                if prediction_record.is_correct {
                    // TODO: Trigger SPL token transfer from our DAO's betting wallet
                    // TODO: Write to web3.storage for permanent storage
                }
        }

        Ok(())
    }

    pub fn checking_it(ctx: Context<CheckingIt>) -> Result<()> {
        let test_record = &mut ctx.accounts.test_record;
        let sender_tokens       = &mut ctx.accounts.user_token_wallet;
        let recipient_tokens    = &mut ctx.accounts.betting_pool_token_wallet;

        test_record.bid_amount                                  = 10 * BID_AMOUNT_EXPONENT;
        test_record.bidder_token_wallet_account_amount          = sender_tokens.amount;
        test_record.betting_pool_token_wallet_account_amount    = recipient_tokens.amount;

        Ok(())
    }
}





#[derive(Accounts)]
pub struct CreatePrediction<'info> {
    #[account(init, payer = user, space = 512)]
    pub prediction_record: Account<'info, PredictionRecord>,

    #[account(mut)] 
     /// CHECK: This is not dangerous because we don't read or write from this account
    pub asset_record:               UncheckedAccount<'info>,    
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub asset_price_record:         UncheckedAccount<'info>,

    #[account(mut)] 
    pub user:                       Signer<'info>,

    #[account(mut)]
    pub mint:                       Account<'info, Mint>,

    #[account(mut)] 
    pub user_token_wallet:          Account<'info, TokenAccount>,

    #[account(mut)] 
    pub betting_pool_token_wallet:  Account<'info, TokenAccount>,    

    pub system_program:             Program<'info, System>,
    pub token_program:              Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ValidatePrediction<'info> {
    #[account(mut)]
    pub prediction_record:  Account<'info, PredictionRecord>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub asset_price_record: UncheckedAccount<'info>,
    pub user:               Signer<'info>,
    pub system_program:     Program<'info, System>,
    pub token_program:      Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CheckingIt<'info> {
    #[account(init, payer = user, space = 512)]
    pub test_record:        Account<'info, TestRecord>,

    #[account(mut)] 
    pub user:               Signer<'info>,

    #[account(mut)]
    pub mint:                       Account<'info, Mint>,

    #[account(mut)] 
    pub user_token_wallet:          Account<'info, TokenAccount>,

    #[account(mut)] 
    pub betting_pool_token_wallet:  Account<'info, TokenAccount>,    


    pub system_program:     Program<'info, System>,    
}

#[account]
pub struct PredictionRecord {
    pub direction: String,
    pub asset: String,
    pub is_correct:bool,
    pub expiry_date: i64,
    pub bidder_token_wallet_key: String,
    pub pyth_product_public_key: String,
    pub pyth_price_public_key: String,
    pub validation_date: i64,
    pub entry_price: i64,
    pub entry_expo: i32,
    pub validation_price: i64,
    pub validation_expo: i32,
    pub bid_amount: u64,

}

#[account]
pub struct TestRecord {
    pub direction: String,
    pub bid_amount: u64,
    pub bidder_token_wallet_account_amount: u64,
    pub betting_pool_token_wallet_account_amount: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient TSD balance.")]
    InsufficientTSD,

    #[msg("Insufficient holdout period.")]
    InsufficientHoldOut,    
}




impl<'info> CreatePrediction<'info> {

    // Return
    fn has_enough_funds(&self, bid_amount: u64) -> bool {
        let sender_tokens       = &self.user_token_wallet;
        let sender_token_balance = sender_tokens.amount;
        let actual_bid_amount = bid_amount * BID_AMOUNT_EXPONENT;

        if sender_token_balance > actual_bid_amount {
            return true;
        } else {
            return false;
        }
    }

    // Transfers the TSD Token from bidder to our Betting Pool
    fn submit_bid(&self, bid_amount: u64) -> bool {
        let sender              = &self.user;
        let sender_tokens       = &self.user_token_wallet;
        let recipient_tokens    = &self.betting_pool_token_wallet;
        let token_program       = &self.token_program;

        // let actual_bid_amount = bid_amount * u64::pow(10, BID_AMOUNT_EXPONENT);
        let actual_bid_amount = bid_amount * BID_AMOUNT_EXPONENT;

        let context = Transfer {
            from:       sender_tokens.to_account_info(),
            to:         recipient_tokens.to_account_info(),
            authority:  sender.to_account_info(),
        };

        token::transfer( 
            CpiContext::new(
                token_program.to_account_info(),
                context
            ),
            actual_bid_amount,
        );

        return true;
    }
}

