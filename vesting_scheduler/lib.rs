#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod vesting_scheduler {
    use ink::primitives::H160;
    use ink::storage::Mapping;

    // Defines a timestamp format
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    pub struct DateTime {
        pub year: u32,
        pub month: u8,
        pub day: u8,
        pub hour: u8,
        pub minute: u8,
        pub second: u8,
    }

    /// Defines a vesting schedule for a beneficiary
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    pub struct VestingSchedule {
        /// Total amount to be vested
        pub total_amount: Balance,
        /// Amount claimed so far
        pub claimed_amount: Balance,
        /// The starting time (should be in ms; we are using Unix epoch)
        pub start_time: u64,
        /// The end time
        pub end_time: u64,
    }

    #[ink(storage)]
    pub struct VestingScheduler {
        /// Maps a beneficiaries address to their vesting schedule
        schedules: Mapping<H160, VestingSchedule>,
        /// Owner of the contract
        owner: H160,
    }

    #[ink(event)]
    pub struct VestingCreated {
        #[ink(topic)]
        beneficiary: H160,
        total_amount: Balance,
        start_time: u64,
        end_time: u64,
    }

    #[ink(event)]
    pub struct TokensClaimed {
        #[ink(topic)]
        beneficiary: H160,
        amount: Balance,
        claimed_at: u64,
    }
    // This event has readable timestamp for demo
    #[ink(event)]
    pub struct TokensClaimedReadable {
        #[ink(topic)]
        beneficiary: H160,
        amount: Balance,
        claimed_at: u64,
        /// Readable format: [Y,Y,Y,Y,-,M,M,-,D,D, ,H,H,:,M,M,:,S,S]
        claimed_at_readable: [u8; 19],
    }
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum Error {
        /// Caller not authorized
        Unauthorized,
        /// Invalid time range
        InvalidTimeRange,
        /// No schedule for a caller
        NoVestingSchedule,
        /// Vesting has not started
        VestingNotStarted,
        /// No tokens available to claim
        NoTokensAvailable,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    impl VestingScheduler {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                schedules: Mapping::default(),
                owner: Self::env().caller(),
            }
        }

        /// Creates a vesting schedule for a beneficiary
        /// `beneficiary` - Account that will receive vested tokens
        /// `total_amount` - Total tokens to vest
        /// `start_time` - Unix timestamp in milliseconds when vesting starts
        /// `end_time` - Unix timestamp in milliseconds when vesting ends
        #[ink(message)]
        pub fn create_vesting_schedule(
            &mut self,
            beneficiary: H160,
            total_amount: Balance,
            start_time: u64,
            end_time: u64,
        ) -> Result<()> {
            if self.env().caller() != self.owner {
                return Err(Error::Unauthorized);
            }
            if start_time >= end_time {
                return Err(Error::InvalidTimeRange);
            }
            let schedule = VestingSchedule {
                total_amount,
                claimed_amount: 0,
                start_time,
                end_time,
            };
            self.schedules.insert(beneficiary, &schedule);
            self.env().emit_event(VestingCreated {
                beneficiary,
                total_amount,
                start_time,
                end_time,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn claim_vested(&mut self) -> Result<Balance> {
            let caller = self.env().caller();
            let current_time = self.env().block_timestamp();

            // Retrieve the vesting schedule
            let mut schedule = self.schedules.get(caller).ok_or(Error::NoVestingSchedule)?;

            // Confirm that vesting has started
            if current_time < schedule.start_time {
                return Err(Error::VestingNotStarted);
            }

            // Calculate vested amount
            let vested_amount = self.calculate_vested_amount(&schedule, current_time);
            let claimable = vested_amount.saturating_sub(schedule.claimed_amount);

            if claimable == 0 {
                return Err(Error::NoTokensAvailable);
            }

            // Update claimed amount
            schedule.claimed_amount = schedule.claimed_amount.saturating_add(claimable);
            self.schedules.insert(caller, &schedule);

            // Emit event(standard event)
            self.env().emit_event(TokensClaimed {
                beneficiary: caller,
                amount: claimable,
                claimed_at: current_time,
            });
            // Emit event with readable timestamp (demonstrates on-chain conversion)
            let dt = self.timestamp_to_datetime(current_time);
            self.env().emit_event(TokensClaimedReadable {
                beneficiary: caller,
                amount: claimable,
                claimed_at: current_time,
                claimed_at_readable: self.format_datetime(dt),
            });

            Ok(claimable)
        }

        /// View function to get vesting schedule with readable dates
        #[ink(message)]
        pub fn get_vesting_schedule_readable(
            &self,
            beneficiary: H160,
        ) -> Option<(VestingSchedule, [u8; 19], [u8; 19])> {
            let schedule = self.schedules.get(beneficiary)?;

            let start_dt = self.timestamp_to_datetime(schedule.start_time);
            let end_dt = self.timestamp_to_datetime(schedule.end_time);

            Some((
                schedule,
                self.format_datetime(start_dt),
                self.format_datetime(end_dt),
            ))
        }

        /// Get vesting schedule (raw timestamps only)
        #[ink(message)]
        pub fn get_vesting_schedule(&self, beneficiary: H160) -> Option<VestingSchedule> {
            self.schedules.get(beneficiary)
        }

        // Timestamp Conversion Functions (no_std compatible)
        /// Convert Unix timestamp (milliseconds) to DateTime
        /// This demonstrates on-chain conversion but is typically done off-chain
        fn timestamp_to_datetime(&self, timestamp_ms: u64) -> DateTime {
            // Convert milliseconds to seconds
            let timestamp = timestamp_ms / 1000;

            // Calculate seconds, minutes, hours
            let second = (timestamp % 60) as u8;
            let minutes_total = timestamp / 60;
            let minute = (minutes_total % 60) as u8;
            let hours_total = minutes_total / 60;
            let hour = (hours_total % 24) as u8;
            let days_total = hours_total / 24;

            // Calculate year (accounting for leap years)
            let mut year = 1970u32;
            let mut remaining_days = days_total;

            // Keep subtracting full years until we have less than 365 days left
            while remaining_days >= 365 {
                let days_in_year = if Self::is_leap_year(year) { 366 } else { 365 };
                remaining_days -= days_in_year;
                year += 1;
            }

            // Calculate month and day
            let (month, day) = Self::days_to_month_day(remaining_days as u32, year);

            DateTime {
                year,
                month,
                day,
                hour,
                minute,
                second,
            }
        }
        /// Check if a year is a leap year
        fn is_leap_year(year: u32) -> bool {
            (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
        }

        /// Convert day of year to month and day
        /// day_of_year is 0-indexed (0 = Jan 1st)
        fn days_to_month_day(day_of_year: u32, year: u32) -> (u8, u8) {
            let is_leap = Self::is_leap_year(year);
            let days_in_months = if is_leap {
                [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
            } else {
                [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
            };

            let mut remaining = day_of_year;
            for (i, &days) in days_in_months.iter().enumerate() {
                if remaining < days {
                    return ((i + 1) as u8, (remaining + 1) as u8);
                }
                remaining = remaining.saturating_sub(days);
            }

            // Fallback (shouldn't reach here with valid input)
            (12, 31)
        }

        /// Format DateTime as a byte array: "YYYY-MM-DD HH:MM:SS"
        /// Note: Returns fixed-size array for no_std compatibility
        fn format_datetime(&self, dt: DateTime) -> [u8; 19] {
            let mut result = [b'0'; 19];

            // Format: YYYY-MM-DD HH:MM:SS
            // Year (4 digits)
            Self::write_u32(&mut result[0..4], dt.year);
            result[4] = b'-';
            // Month (2 digits)
            Self::write_u8(&mut result[5..7], dt.month);
            result[7] = b'-';
            // Day (2 digits)
            Self::write_u8(&mut result[8..10], dt.day);
            result[10] = b' ';
            // Hour (2 digits)
            Self::write_u8(&mut result[11..13], dt.hour);
            result[13] = b':';
            // Minute (2 digits)
            Self::write_u8(&mut result[14..16], dt.minute);
            result[16] = b':';
            // Second (2 digits)
            Self::write_u8(&mut result[17..19], dt.second);

            result
        }

        /// Write a u32 value to a byte buffer as ASCII digits
        fn write_u32(buf: &mut [u8], mut val: u32) {
            for i in (0..buf.len()).rev() {
                buf[i] = b'0' + (val % 10) as u8;
                val /= 10;
            }
        }

        /// Write a u8 value to a 2-byte buffer as ASCII digits
        fn write_u8(buf: &mut [u8], val: u8) {
            buf[0] = b'0' + (val / 10);
            buf[1] = b'0' + (val % 10);
        }

        // Helper functions
        // Calculates the amount vested linearly
        fn calculate_vested_amount(
            &self,
            schedule: &VestingSchedule,
            current_time: u64,
        ) -> Balance {
            if current_time < schedule.start_time {
                return 0;
            }

            if current_time >= schedule.end_time {
                return schedule.total_amount;
            }

            // Linear vesting calculation
            let elapsed = current_time.saturating_sub(schedule.start_time);
            let duration = schedule.end_time.saturating_sub(schedule.start_time);

            // vested = (total * elapsed) / duration
            let vested = (schedule.total_amount as u128)
                .saturating_mul(elapsed as u128)
                .saturating_div(duration as u128) as Balance;

            vested
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn test_vesting_lifecycle() {
            let accounts = ink::env::test::default_accounts();
            // Convert AccountId to H160
            let owner: H160 = accounts.alice.into();
            let beneficiary: H160 = H160::from([1u8; 20]);

            // Set caller to owner BEFORE creating contract
            ink::env::test::set_caller(owner);
            let mut contract = VestingScheduler::new();

            // Set initial block timestamp: Oct 21, 2024, 10:00:00 UTC
            let start_time = 1729512000000u64;
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(start_time);

            // Create vesting schedule: 1M tokens over 100 days
            let total_amount = 1_000_000;
            let end_time = start_time + (100 * 24 * 60 * 60 * 1000); // 100 days later

            let result =
                contract.create_vesting_schedule(beneficiary, total_amount, start_time, end_time);
            assert!(
                result.is_ok(),
                "create_vesting_schedule failed: {:?}",
                result
            );

            // Switch caller to beneficiary to claim
            ink::env::test::set_caller(beneficiary);

            // Advance time by 50 days
            let fifty_days_later = start_time + (50 * 24 * 60 * 60 * 1000);
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(fifty_days_later);

            // Should be able to claim 50% of tokens
            let claimed = contract.claim_vested().unwrap();
            assert_eq!(claimed, 500_000);

            // Advance to after vesting ends
            let after_end = end_time + 1000;
            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(after_end);

            // Should be able to claim remaining 50%
            let remaining = contract.claim_vested().unwrap();
            assert_eq!(remaining, 500_000);

            // No more tokens to claim
            let result = contract.claim_vested();
            assert_eq!(result, Err(Error::NoTokensAvailable));
        }

        #[ink::test]
        fn test_timestamp_conversion() {
            let contract = VestingScheduler::new();

            // Test known timestamp: Oct 21, 2024, 12:00:00 UTC
            let timestamp = 1729512000000u64;
            let dt = contract.timestamp_to_datetime(timestamp);

            assert_eq!(dt.year, 2024);
            assert_eq!(dt.month, 10);
            assert_eq!(dt.day, 21);
            assert_eq!(dt.hour, 12);
            assert_eq!(dt.minute, 0);
            assert_eq!(dt.second, 0);

            // Test formatting
            let formatted = contract.format_datetime(dt);
            let expected = b"2024-10-21 12:00:00";
            assert_eq!(&formatted[..], expected);
        }

        #[ink::test]
        fn test_leap_year() {
            let contract = VestingScheduler::new();

            // Test leap year: Mar 1, 2024
            let leap_day = 1709251200000u64; // 2024-03-01 00:00:00 UTC
            let dt = contract.timestamp_to_datetime(leap_day);

            assert_eq!(dt.year, 2024);
            assert_eq!(dt.month, 3);
            assert_eq!(dt.day, 1);
        }

        #[ink::test]
        fn test_vesting_not_started() {
            let accounts = ink::env::test::default_accounts();
            let owner: H160 = accounts.alice.into(); // Convert AccountId to H160
            let beneficiary: H160 = H160::from([2u8; 20]);

            let current = 1729512000000u64;
            let future_start = current + (10 * 24 * 60 * 60 * 1000); // 10 days from now
            let future_end = future_start + (100 * 24 * 60 * 60 * 1000);

            ink::env::test::set_block_timestamp::<ink::env::DefaultEnvironment>(current);

            // Set caller to owner BEFORE creating contract
            ink::env::test::set_caller(owner);
            let mut contract = VestingScheduler::new();

            let result =
                contract.create_vesting_schedule(beneficiary, 1_000_000, future_start, future_end);
            assert!(
                result.is_ok(),
                "create_vesting_schedule failed: {:?}",
                result
            );

            // Switch to beneficiary to claim
            ink::env::test::set_caller(beneficiary);

            // Try to claim before vesting starts
            let result = contract.claim_vested();
            assert_eq!(result, Err(Error::VestingNotStarted));
        }

        #[ink::test]
        fn test_readable_schedule_view() {
            let accounts = ink::env::test::default_accounts();
            let owner: H160 = accounts.alice.into(); // Convert AccountId to H160
            let beneficiary: H160 = H160::from([3u8; 20]);

            // Set caller to owner BEFORE creating contract
            ink::env::test::set_caller(owner);
            let mut contract = VestingScheduler::new();

            let start = 1729512000000u64; // 2024-10-21 12:00:00
            let end = 1737374400000u64; // 2025-01-20 12:00:00

            let result = contract.create_vesting_schedule(beneficiary, 1_000_000, start, end);
            assert!(
                result.is_ok(),
                "create_vesting_schedule failed: {:?}",
                result
            );

            let result = contract.get_vesting_schedule_readable(beneficiary);
            assert!(result.is_some());

            let (schedule, start_readable, end_readable) = result.unwrap();
            assert_eq!(schedule.total_amount, 1_000_000);
            assert_eq!(&start_readable[..], b"2024-10-21 12:00:00");
            assert_eq!(&end_readable[..], b"2025-01-20 12:00:00");
        }
    }
}
