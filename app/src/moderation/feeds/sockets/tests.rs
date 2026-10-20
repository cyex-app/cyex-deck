//! V107-R17 `plan_all` and `second` against fake sockets: each socket is its subscription
//! state, and Twitch says yes to everything.

use super::*;

#[derive(Default)]
struct Fake {
    welcomed: bool,
    subs: HashMap<String, ChannelSubs>,
    pending: HashSet<String>,
}

fn views(socks: &[Fake]) -> Vec<View<'_>> {
    socks.iter().map(|s| View { session: s.welcomed, subs: &s.subs, pending: &s.pending, in_flight: 0 }).collect()
}

/// Plans, then applies every socket's answer. Returns the spread.
fn step(order: &[String], front: &[&str], socks: &mut [Fake]) -> Spread {
    let front: HashSet<String> = front.iter().map(|c| (*c).to_owned()).collect();
    let spread = plan_all(order, &front, &views(socks), 0);
    for (s, planned) in socks.iter_mut().zip(&spread.per_conn) {
        let Some((drops, adds)) = planned else { continue };
        for (c, i) in drops {
            s.subs.get_mut(c).unwrap().dropped(*i);
        }
        for (n, (c, i)) in adds.iter().enumerate() {
            s.subs.entry(c.clone()).or_default().made_as(*i, format!("{c}-{i}-{n}"), 0);
        }
        assert!(s.subs.values().map(ChannelSubs::held).sum::<usize>() <= subs::MAX_PER_SESSION);
    }
    spread
}

fn chans(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("c{i}")).collect()
}

fn socket_of(socks: &[Fake], cid: &str) -> Vec<usize> {
    socks.iter().enumerate().filter(|(_, s)| s.subs.get(cid).is_some_and(|c| c.held() > 0)).map(|(k, _)| k).collect()
}

fn welcomed() -> Fake {
    Fake { welcomed: true, ..Fake::default() }
}

#[test]
fn v107_r17_one_socket_up_to_150() {
    for n in [10, 27, 100, 150] {
        let mut socks = vec![welcomed()];
        let spread = step(&chans(n), &["c0"], &mut socks);
        assert_eq!((spread.without, spread.unplaced), (0, 0), "{n}");
        assert_eq!(second(1, &spread, false, true), Second::Keep, "{n}: one socket is enough");
    }
}

#[test]
fn v107_r17_second_socket_takes_the_rest() {
    let order = chans(200);
    let mut socks = vec![welcomed()];
    let spread = step(&order, &["c0"], &mut socks);
    assert_eq!((spread.without, spread.unplaced), (50, 50));
    assert_eq!(second(1, &spread, false, true), Second::Open);
    assert_eq!(second(1, &spread, false, false), Second::Keep, "never past Twitch's three");
    // Opened, not welcomed yet: new channels wait, nothing closes.
    socks.push(Fake::default());
    let spread = step(&order, &["c0"], &mut socks);
    assert!(spread.per_conn[1].is_none() && !spread.all_planned);
    assert_eq!(second(2, &spread, false, true), Second::Keep);
    // Welcomed: the 50 left go there, each channel on one socket only.
    socks[1].welcomed = true;
    let spread = step(&order, &["c0"], &mut socks);
    assert_eq!((spread.without, spread.unplaced), (0, 0));
    let (_, adds) = spread.per_conn[1].as_ref().unwrap();
    assert_eq!(adds[0], ("c150".to_owned(), 1));
    for c in &order {
        assert_eq!(socket_of(&socks, c).len(), 1, "{c}");
    }
    assert_eq!(socket_of(&socks, "c199"), vec![1]);
    // 50 channels on the second: 100 AutoMod + 200 of the rest.
    assert_eq!(socks[1].subs.values().map(ChannelSubs::held).sum::<usize>(), 300);
    // Stable: planned again, nothing to do; the second is used.
    let spread = step(&order, &["c0"], &mut socks);
    assert!(spread.per_conn.iter().flatten().all(|(d, a)| d.is_empty() && a.is_empty()));
    assert_eq!(second(2, &spread, true, true), Second::Keep);
    assert_eq!(second(2, &spread, true, false), Second::Close, "whispers or the manager need the slot");
}

#[test]
fn v107_r17_channels_stay_on_their_socket_and_the_second_closes() {
    let order = chans(200);
    let mut socks = vec![welcomed(), welcomed()];
    step(&order, &["c0"], &mut socks);
    // c0..c9 close: the first socket gets room, but c150.. stay on the second.
    let fewer: Vec<String> = order[10..].to_vec();
    let spread = step(&fewer, &["c10"], &mut socks);
    let (_, adds) = spread.per_conn[0].as_ref().unwrap();
    assert!(adds.iter().all(|(c, _)| c[1..].parse::<usize>().unwrap() < 150), "{adds:?}");
    assert_eq!(socket_of(&socks, "c150"), vec![1]);
    // The second socket's channels close: not needed any more.
    let first_only: Vec<String> = order[10..150].to_vec();
    let spread = step(&first_only, &["c10"], &mut socks);
    assert_eq!(spread.unplaced, 0);
    assert_eq!(second(2, &spread, false, true), Second::Close);
}

#[test]
fn v107_r17_shown_channel_on_the_second_socket_gets_its_set_there() {
    let order = chans(200);
    let mut socks = vec![welcomed(), welcomed()];
    step(&order, &["c0"], &mut socks);
    // c199 has AutoMod and part of the rest on the second socket; shown, it gets all 11 there.
    let mut shown = order.clone();
    shown.rotate_right(1);
    step(&shown, &["c199"], &mut socks);
    assert_eq!(socks[1].subs["c199"].held(), subs::MOD_TYPES.len());
    assert_eq!(socket_of(&socks, "c199"), vec![1]);
    assert!(order.iter().all(|c| socks.iter().any(|s| s.subs.get(c).is_some_and(|x| x.id(1).is_some() && x.id(2).is_some()))), "AutoMod kept everywhere");
}
