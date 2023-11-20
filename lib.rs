#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod nostr_ink {

    use ink::prelude::vec::Vec;

    const REWARD_AMOUNT: Balance = 100;
    const PENALTY_AMOUNT: Balance = 50;

    #[ink(storage)]
    pub struct NostrContract {
        owner: AccountId,
        subscription_plans: Vec<SubscriptionPlan>,
        subscriptions: Vec<Subscription>,
        pub reports: Vec<Report>,
        next_report_id: u64,
        challenger: Option<AccountId>,
    }

    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct Subscription {
        subscriber: AccountId,
        relayer_id: AccountId,
        duration: SubscriptionDuration,
    }

    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct SubscriptionPlan {
        relayer_id: AccountId,
        price_per_month: Balance,
        price_per_week: Balance,
        price_per_year: Balance,
        start_date: u64,
        expiry_date: u64,
        subscribers: Vec<AccountId>,
    }

    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum SubscriptionDuration {
        Month,
        Week,
        Year,
        Unknown,
    }

    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct Report {
        pub reporter: AccountId,
        relayer: AccountId,
        pub description: Vec<u8>,
        pub challenged: bool,
        report_id: u64,
    }

    #[derive(PartialEq)]
    #[ink(event)]
    pub struct Subscribed {
        pub subscriber: AccountId,
        pub relayer: AccountId,
        pub amount: Balance,
    }

    #[derive(PartialEq)]
    #[ink(event)]
    pub struct Staked {
        pub staker: AccountId,
        pub relayer: AccountId,
        pub amount: Balance,
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

    #[ink(event)]
    pub struct SubscriptionPlanNotFound {
        #[ink(topic)]
        relayer_id: AccountId,
        #[ink(topic)]
        subscriber: AccountId,
    }

    #[ink(event)]
    pub struct StartDateTimeSet {
        #[ink(topic)]
        start_date: u64,
    }

    #[ink(event)]
    pub struct ExpiryDateTimeSet {
        #[ink(topic)]
        expiry_date: u64,
    }

    impl NostrContract {
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                owner: caller,
                subscriptions: Vec::new(),
                subscription_plans: Vec::new(),
                reports: Vec::new(),
                next_report_id: 1,
                challenger: None,
            }
        }

        #[ink(message)]
        pub fn create_subscription_plan(
            &mut self,
            relayer_id: AccountId,
            price_per_week: Balance,
            price_per_month: Balance,
            price_per_year: Balance,
        ) {
            let start_date = self.env().block_timestamp();

            let expiry_date_monthly = start_date + 30 * 24 * 60 * 60;
            let expiry_date_weekly = start_date + 7 * 24 * 60 * 60;
            let expiry_date_yearly = start_date + 365 * 24 * 60 * 60;

            self.subscription_plans.push(SubscriptionPlan {
                relayer_id,
                price_per_week,
                price_per_month,
                price_per_year,
                start_date,
                expiry_date: expiry_date_monthly,
                subscribers: Vec::new(),
            });

            self.subscription_plans.push(SubscriptionPlan {
                relayer_id,
                price_per_week,
                price_per_month,
                price_per_year,
                start_date,
                expiry_date: expiry_date_weekly,
                subscribers: Vec::new(),
            });

            self.subscription_plans.push(SubscriptionPlan {
                relayer_id,
                price_per_week,
                price_per_month,
                price_per_year,
                start_date,
                expiry_date: expiry_date_yearly,
                subscribers: Vec::new(),
            });
        }

        #[ink(message)]
        pub fn set_start_date(&mut self, plan_index: u32) {
            let current_time = self.env().block_timestamp();
            if let Some(plan) = self.subscription_plans.get_mut(plan_index as usize) {
                plan.start_date = current_time;
                self.env().emit_event(StartDateTimeSet {
                    start_date: current_time,
                });
            }
        }

        #[ink(message)]
        pub fn set_expiry_date(&mut self, plan_index: u32, duration: u64) {
            let expiry_date;

            {
                let plan = self
                    .subscription_plans
                    .get_mut(plan_index as usize)
                    .expect("Invalid plan index");

                expiry_date = plan.start_date + duration;
            }

            if let Some(plan) = self.subscription_plans.get_mut(plan_index as usize) {
                plan.expiry_date = expiry_date;
                ink::env::debug_println!(
                    "{}",
                    ink::prelude::format!("Expiry date updated to: {:?}", expiry_date)
                );
                self.env().emit_event(ExpiryDateTimeSet { expiry_date });
            }
        }

        #[ink(message)]
        pub fn subscribe_to_plan(&mut self, relayer_id: AccountId, duration: SubscriptionDuration) {
            let caller = self.env().caller();
            if let Some(plan) = self
                .subscription_plans
                .iter_mut()
                .find(|p| p.relayer_id == relayer_id)
            {
                match duration {
                    SubscriptionDuration::Month => {
                        plan.subscribers.push(caller);
                        self.subscriptions.push(Subscription {
                            subscriber: caller,
                            relayer_id,
                            duration: SubscriptionDuration::Month,
                        });
                    }
                    SubscriptionDuration::Week => {
                        plan.subscribers.push(caller);
                        self.subscriptions.push(Subscription {
                            subscriber: caller,
                            relayer_id,
                            duration: SubscriptionDuration::Week,
                        });
                    }
                    SubscriptionDuration::Year => {
                        plan.subscribers.push(caller);
                        self.subscriptions.push(Subscription {
                            subscriber: caller,
                            relayer_id,
                            duration: SubscriptionDuration::Year,
                        });
                    }
                    SubscriptionDuration::Unknown => {
                        self.env().emit_event(SubscriptionPlanNotFound {
                            relayer_id,
                            subscriber: caller,
                        });
                    }
                }
            } else {
                self.env().emit_event(SubscriptionPlanNotFound {
                    relayer_id,
                    subscriber: caller,
                });
            }
        }

        #[ink(message)]
        pub fn get_subscription_plans(&self) -> Vec<SubscriptionPlan> {
            self.subscription_plans.clone()
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
                report_id: next_report_id,
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

            if Some(caller) == self.challenger {
                if let Some(report) = self
                    .reports
                    .iter_mut()
                    .find(|r| r.reporter == caller && r.report_id == report_id && !r.challenged)
                {
                    report.challenged = true;

                    if Self::report_is_valid(&report) {
                        // Reward relayer for providing valid data.
                    } else {
                        // Penalize relayer for providing invalid data.
                    }

                    self.env().emit_event(Challenged {
                        reporter: caller,
                        report_id,
                    });
                } else {
                    panic!("Invalid report or challenge request");
                }
            } else {
                panic!("Only the assigned challenger can challenge reports");
            }
        }

        fn report_is_valid(report: &Report) -> bool {
            if !report.description.is_empty() && !report.challenged {
                true
            } else {
                false
            }
        }

        #[ink(message)]
        pub fn get_subscription(
            &self,
            relayer_id: AccountId,
            subscriber: AccountId,
        ) -> Option<Subscription> {
            self.subscription_plans
                .iter()
                .flat_map(|plan| plan.subscribers.iter().map(move |sub| (plan, *sub)))
                .find(|(plan, sub)| plan.relayer_id == relayer_id && *sub == subscriber)
                .map(|(plan, expiry_date)| {
                    let duration = match plan.price_per_month {
                        0 => SubscriptionDuration::Unknown,
                        price => {
                            let now = self.env().block_timestamp();
                            if plan.expiry_date >= now {
                                if plan.price_per_week > 0 {
                                    SubscriptionDuration::Week
                                } else if plan.price_per_year > 0 {
                                    SubscriptionDuration::Year
                                } else {
                                    SubscriptionDuration::Month
                                }
                            } else {
                                SubscriptionDuration::Unknown
                            }
                        }
                    };

                    Subscription {
                        relayer_id: plan.relayer_id,
                        duration,
                        subscriber,
                    }
                })
        }

        #[ink(message)]
        pub fn get_subscribers(&self, relayer_id: AccountId) -> Vec<AccountId> {
            // Find the subscription plan for the given relayer
            if let Some(plan) = self
                .subscription_plans
                .iter()
                .find(|p| p.relayer_id == relayer_id)
            {
                // Return the list of subscribers for the found subscription plan
                plan.subscribers.clone()
            } else {
                // If the subscription plan for the relayer is not found, return an empty vector
                Vec::new()
            }
        }

        #[ink(message)]
        pub fn get_report(&self, report_id: u64) -> Option<Report> {
            self.reports
                .iter()
                .find(|r| r.report_id == report_id)
                .cloned()
        }
    }

    #[cfg(test)]
    mod tests {}
}
