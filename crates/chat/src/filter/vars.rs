//! The names a filter can read, with Chatterino's names so filters can be shared, plus
//! a few of ours (`author.login`, `message.client`, `flags.mention` …).

use crate::model::{ChatMessage, Client, Flag, Fragment, NoticeKind, plain};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Bool,
    Int,
    Str,
    List,
    Regex,
}

impl Type {
    pub fn name(self) -> &'static str {
        match self {
            Type::Bool => "true/false",
            Type::Int => "a number",
            Type::Str => "text",
            Type::List => "a list",
            Type::Regex => "a regex",
        }
    }
}

macro_rules! vars {
    ($($v:ident = $name:literal : $ty:ident),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Var { $($v),* }

        impl Var {
            pub const ALL: &[(Var, &str)] = &[$((Var::$v, $name)),*];

            pub fn from_name(name: &str) -> Option<Var> {
                match name { $($name => Some(Var::$v),)* _ => None }
            }

            pub fn ty(self) -> Type {
                match self { $(Var::$v => Type::$ty),* }
            }
        }
    };
}

vars! {
    AuthorBadges = "author.badges": List,
    AuthorExternalBadges = "author.external_badges": List,
    AuthorColor = "author.color": Str,
    AuthorName = "author.name": Str,
    AuthorLogin = "author.login": Str,
    AuthorUserId = "author.user_id": Str,
    AuthorNoColor = "author.no_color": Bool,
    AuthorSubbed = "author.subbed": Bool,
    AuthorSubLength = "author.sub_length": Int,
    BitsAmount = "bits.amount": Int,
    ChannelName = "channel.name": Str,
    ChannelLive = "channel.live": Bool,
    ChannelWatching = "channel.watching": Bool,
    Action = "flags.action": Bool,
    Highlighted = "flags.highlighted": Bool,
    PointsRedeemed = "flags.points_redeemed": Bool,
    SubMessage = "flags.sub_message": Bool,
    SystemMessage = "flags.system_message": Bool,
    RewardMessage = "flags.reward_message": Bool,
    FirstMessage = "flags.first_message": Bool,
    ElevatedMessage = "flags.elevated_message": Bool,
    HypeChat = "flags.hype_chat": Bool,
    CheerMessage = "flags.cheer_message": Bool,
    Whisper = "flags.whisper": Bool,
    Reply = "flags.reply": Bool,
    Automod = "flags.automod": Bool,
    Restricted = "flags.restricted": Bool,
    Monitored = "flags.monitored": Bool,
    Shared = "flags.shared": Bool,
    Similar = "flags.similar": Bool,
    WatchStreak = "flags.watch_streak": Bool,
    Announcement = "flags.announcement": Bool,
    Mention = "flags.mention": Bool,
    Own = "flags.own": Bool,
    History = "flags.history": Bool,
    Deleted = "flags.deleted": Bool,
    Repeated = "flags.repeated": Bool,
    PowerUp = "flags.power_up": Bool,
    MessageContent = "message.content": Str,
    MessageLength = "message.length": Int,
    MessageClient = "message.client": Str,
    RewardTitle = "reward.title": Str,
    RewardCost = "reward.cost": Int,
    RewardId = "reward.id": Str,
}

/// What a filter looks at: a message and where it is.
pub struct Facts<'a> {
    pub msg: &'a ChatMessage,
    /// Channel login.
    pub channel: &'a str,
    pub live: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Str(String),
    List(Vec<Value>),
}

impl Facts<'_> {
    pub fn get(&self, v: Var) -> Value {
        let m = self.msg;
        let s = m.sender.as_ref();
        let has = |f: Flag| Value::Bool(m.has(f));
        let kind = |k: NoticeKind| Value::Bool(m.notice.as_ref().is_some_and(|n| n.kind == k));
        let badge = |set: &str| s.and_then(|s| s.badges.iter().find(|b| b.set == set));
        match v {
            Var::AuthorBadges => Value::List(
                s.map(|s| s.badges.iter().map(|b| Value::Str(b.set.clone())).collect())
                    .unwrap_or_default(),
            ),
            Var::AuthorExternalBadges => Value::List(Vec::new()),
            Var::AuthorColor => Value::Str(s.map(|s| s.color.dark.clone()).unwrap_or_default()),
            Var::AuthorName => Value::Str(s.map(|s| s.display.clone()).unwrap_or_default()),
            Var::AuthorLogin => Value::Str(s.map(|s| s.login.clone()).unwrap_or_default()),
            Var::AuthorUserId => Value::Str(s.map(|s| s.id.clone()).unwrap_or_default()),
            Var::AuthorNoColor => Value::Bool(false),
            Var::AuthorSubbed => Value::Bool(badge("subscriber").or_else(|| badge("founder")).is_some()),
            Var::AuthorSubLength => Value::Int(
                badge("subscriber")
                    .or_else(|| badge("founder"))
                    .and_then(|b| b.info.as_deref()?.parse().ok())
                    .unwrap_or(0),
            ),
            Var::BitsAmount => Value::Int(bits(m)),
            Var::ChannelName => Value::Str(self.channel.to_owned()),
            Var::ChannelLive => Value::Bool(self.live),
            Var::ChannelWatching | Var::ElevatedMessage | Var::HypeChat | Var::Automod => Value::Bool(false),
            Var::Whisper => has(Flag::Whisper),
            Var::Restricted | Var::Monitored => Value::Bool(false),
            Var::Action => has(Flag::Action),
            Var::Highlighted => Value::Bool(m.meta.highlight.is_some()),
            Var::PointsRedeemed => has(Flag::Highlighted),
            Var::SubMessage => kind(NoticeKind::Sub),
            Var::SystemMessage => Value::Bool(m.notice.is_some() && m.sender.is_none()),
            Var::RewardMessage => Value::Bool(m.reward.is_some()),
            Var::FirstMessage => has(Flag::FirstMessage),
            Var::CheerMessage => Value::Bool(bits(m) > 0),
            Var::Reply => Value::Bool(m.reply.is_some()),
            Var::Shared => Value::Bool(m.source.is_some()),
            Var::Similar => has(Flag::Similar),
            Var::WatchStreak => kind(NoticeKind::Milestone),
            Var::Announcement => kind(NoticeKind::Announcement),
            Var::Mention => has(Flag::Mention),
            Var::Own => has(Flag::Own),
            Var::History => has(Flag::History),
            Var::Deleted => has(Flag::Deleted),
            Var::Repeated => Value::Bool(m.meta.repeat.is_some()),
            Var::PowerUp => Value::Bool(m.has(Flag::Gigantified) || m.has(Flag::Animated)),
            Var::MessageContent => Value::Str(plain(&m.body)),
            Var::MessageLength => Value::Int(plain(&m.body).chars().count() as i64),
            Var::MessageClient => Value::Str(
                match m.meta.client {
                    None => "unknown",
                    Some(Client::Web) => "web",
                    Some(Client::Android) => "android",
                    Some(Client::Ios) => "ios",
                    Some(Client::Mobile) => "mobile",
                    Some(Client::Odd) => "odd",
                }
                .into(),
            ),
            Var::RewardTitle => Value::Str(
                m.reward
                    .as_ref()
                    .and_then(|r| r.title.clone())
                    .unwrap_or_default(),
            ),
            Var::RewardCost => Value::Int(m.reward.as_ref().and_then(|r| r.cost).map_or(-1, i64::from)),
            Var::RewardId => Value::Str(m.reward.as_ref().map(|r| r.id.clone()).unwrap_or_default()),
        }
    }
}

fn bits(m: &ChatMessage) -> i64 {
    m.body
        .iter()
        .map(|f| match f {
            Fragment::Cheer { amount, .. } => i64::from(*amount),
            _ => 0,
        })
        .sum()
}
