Vesting Scheduler
A smart contract for managing token vesting schedules with human-readable timestamps. This ink! smart contract allows creating linear vesting schedules and claiming vested tokens over time.

Features
Create vesting schedules for beneficiaries
Linear token vesting over specified time periods
Claim vested tokens as they become available
Human-readable timestamp conversion (Unix to YYYY-MM-DD HH:MM:SS format)
Comprehensive test coverage
How to Run
Prerequisites
Rust toolchain (>= 1.88)
Cargo
cargo-contract (for building ink! contracts)

# Install cargo-contract

cargo install --force --locked --tag v6.0.0-alpha.4 --git https://github.com/use-ink/cargo-contract

# Update rust

rustup update stable

# Install an ink-node (for local testing)

# Download from: https://github.com/use-ink/ink-node/releases

For detailed setup instructions, see the official ink! v6 Setup Guide

Build and Test

# Build the contract

cargo contract build

# Run tests

cargo contract test

Deploy
This is an ink! smart contract that can be deployed to any Substrate-based blockchain that supports ink! contracts.

Usage
The contract provides these main functions:

create_vesting_schedule() - Create a new vesting schedule (owner only)
claim_vested() - Claim available vested tokens
get_vesting_schedule_readable() - View schedule with human-readable dates
get_vesting_schedule() - View raw schedule data
