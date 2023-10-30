#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod nostr_contract {

    #[ink(storage)]
    pub struct NostrContract {
        owner: AccountId,
        subscriptions: Vec<Subscription>,
        pub relayer_stakes: Vec<(AccountId, Balance)>,
        pub reports: Vec<Report>,
        next_report_id: u64,
        challenger: Option<AccountId>,
    }

    #[derive(Debug, Clone, scale::Encode, scale::Decode, scale_info::TypeInfo)]
    pub struct Subscription {
        relayer: AccountId,
        amount: Balance,
    }

    #[derive(Debug, Clone, scale::Encode, scale::Decode, scale_info::TypeInfo)]
    pub struct Report {
        pub reporter: AccountId,
        relayer: AccountId,
        pub description: Vec<u8>,
        pub challenged: bool,
    }

    #[ink(event)]
    pub struct Subscribed {
        subscriber: AccountId,
        relayer: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct Staked {
        staker: AccountId,
        relayer: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct Reported {
        reporter: AccountId,
        relayer: AccountId,
        pub description: Vec<u8>,
        report_id: u64,
    }

    #[ink(event)]
    pub struct Challenged {
        reporter: AccountId,
        report_id: u64,
    }

    impl NostrContract {
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                owner: caller,
                subscriptions: Vec::new(),
                relayer_stakes: Vec::new(),
                reports: Vec::new(),
                next_report_id: 1,
                challenger: None,
            }
        }

        #[ink(message)]
        pub fn subscribe(&mut self, relayer: AccountId, amount: Balance) {
            let caller = self.env().caller();
            assert_ne!(caller, relayer, "You cannot subscribe to yourself");

            self.subscriptions.push(Subscription { relayer, amount });

            self.env().emit_event(Subscribed {
                subscriber: caller,
                relayer,
                amount,
            });
        }

        #[ink(message)]
        pub fn stake(&mut self) {
            let caller = self.env().caller();
            if let Some(subscription) = self.subscriptions.last() {
                let (relayer, amount) = (subscription.relayer, subscription.amount);

                let existing_stake =
                    match self.relayer_stakes.iter_mut().find(|(r, _)| *r == relayer) {
                        Some((_, s)) => s,
                        None => {
                            self.relayer_stakes.push((relayer, 0)); // Add a new stake entry
                            &mut self.relayer_stakes.last_mut().unwrap().1 // Borrow the mutable reference
                        }
                    };

                *existing_stake += amount;

                self.env().emit_event(Staked {
                    staker: caller,
                    relayer,
                    amount,
                });
            }
        }
        #[ink(message)]
        pub fn report(&mut self, relayer: AccountId, report_id: u64, reporter: AccountId, description: Vec<u8>) {
            let caller = self.env().caller();

            let next_report_id = self.next_report_id;
            self.next_report_id += 1; // Increment the next report ID

            self.reports.push(Report {
                reporter: caller,
                relayer,
                description: description.clone(),
                challenged: false,
            });

            self.env().emit_event(Reported {
                reporter: caller,
                relayer,
                description,
                report_id: next_report_id,
            });
        }

        #[ink(message)]
        pub fn set_challenger(&mut self, challenger: AccountId) {
            // Implement a setter function to set the challenger account
            self.challenger = Some(challenger);
        }

        #[ink(message)]
        pub fn challenge(&mut self, report_id: u64, challenger: AccountId) {
            let caller = self.env().caller();
    
            // Check if the caller is the stored challenger account
            if Some(caller) == self.challenger {
                let report = self
                    .reports
                    .iter_mut()
                    .find(|r| r.reporter == caller && r.challenged == false)
                    .unwrap();
    
                report.challenged = true;
    
                self.env().emit_event(Challenged {
                    reporter: caller,
                    report_id,
                });
            } else {
                // Handle unauthorized challenges
                // For example: Emit an event or revert the transaction
                // You can implement your specific logic here
            }
        }
    
    }
}

#[cfg(test)]
mod tests {
    use crate::nostr_contract::NostrContract;

    use ink::env::{test, DefaultEnvironment};

    use super::*;

    #[ink::test]
    fn test_subscribe_and_stake() {
        // Deploy the contract
        let mut contract = NostrContract::new();

        // Get accounts
        let accounts = test::default_accounts::<DefaultEnvironment>();

        // Subscribe and stake
        contract.subscribe(accounts.bob, 100);
        contract.stake();

        // Get the relayer stakes
        let relayer_stakes = contract.relayer_stakes;
        assert_eq!(relayer_stakes.len(), 1);
        assert_eq!(relayer_stakes[0].0, accounts.bob);
        assert_eq!(relayer_stakes[0].1, 100);
    }

    #[ink::test]
    fn test_report_and_challenge() {
        // Use the test environment to set up test accounts.
        let accounts = test::default_accounts::<DefaultEnvironment>();
    
        // Deploy the contract
        let mut contract = NostrContract::new();


        // Subscribe and stake
        contract.subscribe(accounts.alice, 100);
        contract.stake();
    
        // Report an issue with one account (e.g., accounts.bob)
        // contract.report(accounts.bob, vec![1, 2, 3]);
    
        // Set the challenger account within the contract
        contract.set_challenger(accounts.bob);
    
        // Challenge the report
        contract.challenge(1, accounts.bob);
    
        // Get the challenged report
        let reports = &contract.reports;
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].reporter, accounts.bob); // Ensure the reporter matches the account used for reporting
        assert_eq!(reports[0].challenged, true);
    
    }
    
}
