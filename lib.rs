#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod nostr_ink {

    use ink::prelude::vec::Vec;

    use ink::env::{
        DefaultEnvironment,
        // Environment,
    };

    use ink::trait_definition;
    use pallet_contracts::Schedule;
    const PENALTY_AMOUNT: Balance = 50;

    #[derive(Debug, Clone, Default)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct SubscriberInfo {
        pub ok: Vec<SubscriberInfoEntry>,
    }

    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct SubscriberInfoEntry {
        pub sub_id: AccountId,
        pub nostr_pubkey: Vec<u8>,
        duration: SubscriptionDuration,
        start_date: u64,
        expiry_date: u64,
    }

    #[ink(event)]
    pub struct SubscriptionExpired {
        #[ink(topic)]
        subscriber: AccountId,
        expiry_date: u64,
    }

    #[ink(event)]
    pub struct PenaltyApplied {
        #[ink(topic)]
        relayer_id: AccountId,
        amount: Balance,
    }

    #[ink(storage)]
    pub struct NostrContract {
        owner: AccountId,
        subscription_plans: Vec<SubscriptionPlan>,
        subscriptions: Vec<Subscription>,
        pub reports: Vec<Report>,
        next_report_id: u64,
        challenger: Option<AccountId>,
        pub nostr_public_keys: Vec<(AccountId, Vec<u8>)>,
        next_plan_id: u64,
        total_earnings: Balance,
    }

    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct Subscription {
        subscriber: AccountId,
        relayer_id: AccountId,
        duration: SubscriptionDuration,
        start_date: u64,
        expiry_date: u64,
    }

    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct SubscriptionPlan {
        relayer_id: AccountId,
        subscribers: Vec<AccountId>,
        plan_id: u64,
        price: Balance,
        duration: SubscriptionDuration,
    }

    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct SubscriptionPlanInfo {
        pub relayer_id: AccountId,
        pub price: PriceInfo,
        pub plan_id: PlanId,
    }

    #[derive(Debug, Clone, Default)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct PriceInfo {
        pub price_per_month: Option<Balance>,
        pub price_per_week: Option<Balance>,
        pub price_per_year: Option<Balance>,
    }

    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct PlansInfo {
        pub relayer_id: AccountId,
        pub duration: SubscriptionDuration,
    }

    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct SubscriptionPlanDuration {
        pub duration: SubscriptionDuration,
        pub price: Balance,
    }

    #[derive(Debug, Clone, Default)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct PlanId {
        pub week_plan_id: Option<u64>,
        pub month_plan_id: Option<u64>,
        pub year_plan_id: Option<u64>,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum SubscriptionDuration {
        Month(Balance),
        Week(Balance),
        Year(Balance),
        Unknown,
    }

    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct Report {
        pub reporter: AccountId,
        pub relayer: AccountId,
        pub challenged: bool,
        pub report_id: u64,
    }

    #[derive(PartialEq)]
    #[ink(event)]
    pub struct Subscribed {
        pub subscriber: AccountId,
        pub relayer: AccountId,
        pub amount: Balance,
    }

    #[ink(event)]
    pub struct ReportEvent {
        #[ink(topic)]
        pub relayer: AccountId,
        pub receipt: Vec<u8>,
        pub report_id: u64,
    }

    #[ink(event)]
    pub struct Challenged {
        reporter: AccountId,
        report_id: u64,
    }

    #[ink(event)]
    pub struct SubscriptionPlanNotFound {
        #[ink(topic)]
        not_found_id: AccountId,
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
                nostr_public_keys: Vec::new(),
                next_report_id: 1,
                challenger: None,
                next_plan_id: 1,
                total_earnings: 0,
            }
        }
        #[ink(message)]
        pub fn create_subscription_plan(
            &mut self,
            duration: SubscriptionDuration,
        ) -> (u64, PlansInfo) {
            let caller = self.env().caller();
            let relayer_id = caller; // set the relayer_id to the caller

            let plan_id = self.next_plan_id;
            self.next_plan_id += 1;

            let price = match duration {
                SubscriptionDuration::Month(price) => price,
                SubscriptionDuration::Week(price) => price,
                SubscriptionDuration::Year(price) => price,
                SubscriptionDuration::Unknown => 0,
            };

            let plan = SubscriptionPlan {
                relayer_id,
                subscribers: Vec::new(),
                price,
                duration,
                plan_id,
            };
            self.subscription_plans.push(plan);

            let plan_info = PlansInfo {
                relayer_id,
                duration,
            };

            (plan_id, plan_info)
        }

        #[ink(message)]
        pub fn subscribe_to_plan(&mut self, plan_id: u64, nostr_public_key: Vec<u8>) {
            let subscriber = self.env().caller(); // Set the subscriber to the caller

            let current_time = self.env().block_timestamp();
            let mut found_plan: Option<&mut SubscriptionPlan> = None;

            // Find the subscription plan with the given plan_id
            for plan in &mut self.subscription_plans {
                if plan.plan_id == plan_id {
                    found_plan = Some(plan);
                    break;
                }
            }

            if let Some(plan) = found_plan {
                let relayer_id = plan.relayer_id;
                let duration = plan.duration.clone();
                let price = plan.price;

                let (start_date, expiry_date) = match duration {
                    SubscriptionDuration::Month(_) => {
                        (current_time, current_time + 30 * 24 * 60 * 60)
                    }
                    SubscriptionDuration::Week(_) => {
                        (current_time, current_time + 7 * 24 * 60 * 60)
                    }
                    SubscriptionDuration::Year(_) => {
                        (current_time, current_time + 365 * 24 * 60 * 60)
                    }
                    SubscriptionDuration::Unknown => (0, 0),
                };

                let subscription = Subscription {
                    subscriber,
                    relayer_id,
                    duration,
                    start_date,
                    expiry_date,
                };

                self.subscriptions.push(subscription);

                // Add the subscriber's ID to the subscribers vector of the subscription plan
                plan.subscribers.push(subscriber);

                self.env().emit_event(Subscribed {
                    subscriber,
                    relayer: relayer_id,
                    amount: price,
                });

                self.nostr_public_keys.push((subscriber, nostr_public_key));
            } else {
                // If the plan is not found, emit an event indicating the subscription plan was not found
                self.env().emit_event(SubscriptionPlanNotFound {
                    not_found_id: subscriber,
                    relayer_id: AccountId::from([0; 32]),
                    subscriber,
                });
            }
        }

        #[ink(message)]
        pub fn get_subscription_plans(&self) -> Vec<(u64, PlansInfo)> {
            self.subscription_plans
                .iter()
                .map(|plan| {
                    (
                        plan.plan_id,
                        PlansInfo {
                            relayer_id: plan.relayer_id,
                            duration: plan.duration.clone(),
                        },
                    )
                })
                .collect()
        }

        #[ink(message)]
        pub fn pay_relayer(&mut self) {
            let current_time = self.env().block_timestamp();

            // Iterate through expired subscriptions
            for subscription in &self.subscriptions {
                if subscription.expiry_date <= current_time {
                    let plan = self
                        .subscription_plans
                        .iter()
                        .find(|p| p.relayer_id == subscription.relayer_id)
                        .expect("Subscription plan not found");

                    // Calculate earnings for the expired subscription
                    let earnings = (plan.price * 70) / 100;

                    // Transfer earnings to the relayer
                    self.env()
                        .transfer(subscription.relayer_id, earnings)
                        .expect("Transfer failed");
                }
            }
        }

        #[ink(message)]
        pub fn report(&mut self, relayer_bytes: Vec<u8>, receipt: Vec<u8>) {
            let caller = self.env().caller();

            let next_report_id = self.next_report_id;
            self.next_report_id += 1;

            // Decode the relayer's bytes to get the relayer's AccountId
            let relayer = self.decode_account_id(&relayer_bytes);

            self.reports.push(Report {
                reporter: caller,
                relayer,
                challenged: false,
                report_id: next_report_id,
            });

            // Emit the ReportEvent event with the relayer's bytes and receipt
            self.env().emit_event(ReportEvent {
                relayer,
                receipt,
                report_id: next_report_id,
            });

            // Schedule the task to listen for challenges
            self.listen_for_challenges();
        }

        fn decode_account_id(&self, bytes: &[u8]) -> AccountId {
            assert_eq!(bytes.len(), 32, "Invalid length for AccountId bytes");

            let mut array = [0u8; 32];
            array.copy_from_slice(bytes);

            AccountId::from(array)
        }

        #[ink(message)]
        pub fn get_subscribers(&self) -> SubscriberInfo {
            let relayer_id = self.env().caller();

            // Find the subscription plan for the given relayer
            if let Some(plan) = self
                .subscription_plans
                .iter()
                .find(|p| p.relayer_id == relayer_id)
            {
                let subscribers_info: Vec<SubscriberInfoEntry> = plan
                    .subscribers
                    .iter()
                    .filter_map(|&subscriber| {
                        if let Some(subscription) = self
                            .subscriptions
                            .iter()
                            .find(|&sub| sub.subscriber == subscriber)
                        {
                            let start_date = subscription.start_date;
                            let expiry_date = subscription.expiry_date;

                            // Retrieve the Nostr public key for the current subscriber
                            if let Some((id, nostr_key)) = self
                                .nostr_public_keys
                                .iter()
                                .find(|&&(id, _)| id == subscriber)
                            {
                                Some(SubscriberInfoEntry {
                                    sub_id: *id,
                                    nostr_pubkey: nostr_key.clone(),
                                    duration: plan.duration.clone(),
                                    start_date,
                                    expiry_date,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                SubscriberInfo {
                    ok: subscribers_info,
                }
            } else {
                SubscriberInfo { ok: Vec::new() }
            }
        }

        #[ink(message)]
        pub fn get_total_earnings(&self) -> Balance {
            let caller = self.env().caller();
            let relayer_id = self.env().caller();
            let mut total_earnings: Balance = 0;

            // Iterate through each subscription plan
            for plan in self.subscription_plans.iter() {
                if plan.relayer_id == relayer_id {
                    for subscriber_plan in self
                        .subscriptions
                        .iter()
                        .filter(|&sub| sub.relayer_id == caller)
                    {
                        if subscriber_plan.duration == plan.duration
                            && subscriber_plan.start_date <= self.env().block_timestamp()
                            && subscriber_plan.expiry_date >= self.env().block_timestamp()
                        {
                            total_earnings += plan.price;
                        }
                    }
                }
            }

            total_earnings
        }

        #[ink(message)]
        pub fn listen_for_challenges(&mut self) {
            // Schedule a task to listen for challenges periodically
            // let schedule = self.env().extension::<pallet_scheduler::Scheduled>().expect("Scheduler not available");

            // let task = pallet_scheduler::ScheduleTask {
            //     delay: 10,
            //     period: 20,
            //     calls: vec![pallet_scheduler::ScheduleCall {
            //         method: Self::handle_challenge_task as _,
            //         data: vec![],
            //     }],
            // };

            // Schedule the named task to listen for challenges periodically
            // schedule.schedule_named(b"challenge_task".to_vec(), task);
        }

        pub fn handle_challenge_task(&mut self) {
            let current_time = self.env().block_timestamp();
            let challenge_expiry_time = current_time - (60 * 60);

            let relayer_to_penalize = {
                let mut relayer_opt: Option<AccountId> = None;
                for report in self.reports.iter_mut() {
                    if !report.challenged && report.report_id == self.next_report_id - 1 {
                        if current_time > challenge_expiry_time {
                            // Store the relayer ID to penalize
                            relayer_opt = Some(report.relayer);
                        }
                    }
                }
                relayer_opt
            };

            if let Some(relayer) = relayer_to_penalize {
                self.penalize_relayer(relayer);
            }
        }

        fn penalize_relayer(&mut self, relayer_id: AccountId) {
            let mut total_penalized_amount: Balance = 0;

            // Iterate through subscriptions to find the relayer's plans
            for subscription in &self.subscriptions {
                if subscription.relayer_id == relayer_id {
                    if let Some(plan) = self
                        .subscription_plans
                        .iter()
                        .find(|p| p.relayer_id == relayer_id)
                    {
                        // Deduct 50% of the plan's price as penalty
                        let penalty_amount = plan.price / 2;
                        total_penalized_amount += penalty_amount;

                        // Transfer the penalty amount to the contract's account
                        self.env()
                            .transfer(relayer_id, penalty_amount)
                            .expect("Transfer failed");
                    }
                }
            }

            // Emit an event indicating the penalty
            self.env().emit_event(PenaltyApplied {
                relayer_id,
                amount: total_penalized_amount,
            });
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
    mod tests {
        use ink::primitives::AccountId;

        use crate::nostr_ink::{NostrContract, SubscriptionDuration};

        #[ink::test]
        fn test_create_subscription_plan() {
            // Initialize the contract
            let mut contract = NostrContract::new();

            // Alice creates a subscription plan for a month
            let (plan_id, _) = contract.create_subscription_plan(SubscriptionDuration::Month(100));

            let plans_info = contract.get_subscription_plans();
            assert_eq!(plans_info.len(), 1);
            assert_eq!(plans_info[0].0, plan_id);
            assert_eq!(plans_info[0].1.duration, SubscriptionDuration::Month(100));
        }

        #[ink::test]
        fn test_subscribe_to_plan() {
            // Initialize the contract
            let mut contract = NostrContract::new();
            let bob = AccountId::from([1; 32]);

            let (plan_id, _) = contract.create_subscription_plan(SubscriptionDuration::Month(100));

            // Bob subscribes to the plan
            contract.subscribe_to_plan(plan_id, vec![1, 2, 3]);

            let subscribers_info = contract.get_subscribers();
            assert_eq!(subscribers_info.ok.len(), 1);
            assert_eq!(subscribers_info.ok[0].sub_id, bob);
            assert_eq!(
                subscribers_info.ok[0].duration,
                SubscriptionDuration::Month(100)
            );
        }

        #[ink::test]
        fn test_report() {
            // Initialize the contract
            let mut contract = NostrContract::new();
            let bob = AccountId::from([1; 32]);

            // Bob reports a relayer
            contract.report(vec![3; 32], vec![4, 5, 6]);

            let report = contract.get_report(1).unwrap();
            assert_eq!(report.reporter, bob);
            assert_eq!(report.challenged, false);
        }

    }
}
