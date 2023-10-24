#![cfg_attr(not(feature = "std"), no_std)]


#[ink::contract]
mod nostr_contract {
    use ink::storage::Lazy;

    #[ink(storage)]
    pub struct NostrContract {
        owner: AccountId,
        subscriptions: Vec<Subscription>,
        relayer_stakes: Vec<(AccountId, Balance)>,
        reports: Vec<Report>,
        next_report_id: u64,
    }

    #[derive(Debug, Clone, scale::Encode, scale::Decode, scale_info::TypeInfo)]
    pub struct Subscription {
        relayer: AccountId,
        amount: Balance,
    }

    #[derive(Debug, Clone, scale::Encode, scale::Decode, scale_info::TypeInfo)]
    pub struct Report {
        reporter: AccountId,
        relayer: AccountId,
        description: Vec<u8>,
        challenged: bool,
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
        description: Vec<u8>,
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
            }
        }

        #[ink(message)]
        pub fn subscribe(&mut self, relayer: AccountId, amount: Balance) {
            let caller = self.env().caller();
            assert_ne!(caller, relayer, "You cannot subscribe to yourself");

            self.subscriptions.push(Subscription {
                relayer,
                amount,
            });

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
        
                let existing_stake = match self.relayer_stakes.iter_mut().find(|(r, _)| *r == relayer) {
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
        pub fn report(&mut self, relayer: AccountId, description: Vec<u8>) {
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
        pub fn challenge(&mut self, report_id: u64) {
            let caller = self.env().caller();
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
        }
    }
}

    #[cfg(test)]
    mod tests {
   
    }

