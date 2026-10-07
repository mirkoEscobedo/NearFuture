// Low-level custody regressions stay private; actual public behavior is tested via NotifyLane.
#[path = "../../../tests/notification_internal/broker.rs"]
mod broker;
#[path = "../../../tests/notification_internal/delivery.rs"]
mod delivery;
#[path = "../../../tests/notification_internal/notice.rs"]
mod notice;
#[path = "../../../tests/notification_effect_support/mod.rs"]
mod support;

#[path = "../../../tests/notification_internal/deadline.rs"]
mod deadline;

#[path = "../../../tests/notification_internal/auth.rs"]
mod auth;

#[path = "../../../tests/notification_internal/subscription.rs"]
mod subscription;

#[path = "../../../tests/notification_internal/queue.rs"]
mod queue;

#[path = "../../../tests/notification_internal/rate.rs"]
mod rate;

#[path = "../../../tests/notification_internal/completion_deadline.rs"]
mod completion_deadline;

#[path = "../../../tests/notification_internal/support_sessions.rs"]
mod support_sessions;

#[path = "../../../tests/notification_internal/read_cuts.rs"]
mod read_cuts;

#[path = "../../../tests/notification_internal/final_completion.rs"]
mod final_completion;
