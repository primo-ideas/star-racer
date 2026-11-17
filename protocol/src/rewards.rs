//! Rewards: the more a player races, the more they unlock.
//!
//! Every race pays experience (XP), more for finishing and for every player beaten, and XP makes
//! levels. Some levels unlock something for the garage: of the six cars, six trails and six horns,
//! the first three of each are free and the other three are won this way, a horn first, then a
//! trail, then a car, three times over. Nothing is bought, nothing is lost, and the levels go on
//! past the last reward. The rules live here so the server, which keeps every player's XP and
//! refuses a locked look, and the client, which shows what is locked and what was just won, agree.

use crate::{CAR_HORNS, CAR_MODELS, CAR_TRAILS, CarLook};

/// How many of each kind are free from the start.
pub const FREE: u8 = 3;

/// Something to win.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reward {
    Car(u8),
    Trail(u8),
    Horn(u8),
}

/// What each level unlocks, in order.
pub const REWARDS: [(u32, Reward); 9] = [
    (2, Reward::Horn(3)),
    (3, Reward::Trail(3)),
    (4, Reward::Car(3)),
    (5, Reward::Horn(4)),
    (6, Reward::Trail(4)),
    (8, Reward::Car(4)),
    (9, Reward::Horn(5)),
    (10, Reward::Trail(5)),
    (12, Reward::Car(5)),
];

/// XP from one level to the next: the first level costs [`LEVEL_BASE`], each one after it
/// [`LEVEL_STEP`] more.
const LEVEL_BASE: u32 = 300;
const LEVEL_STEP: u32 = 60;

/// The XP a race pays: for taking part, for finishing, for each player finished ahead of, and for
/// winning against at least one other.
const XP_RACED: u32 = 60;
const XP_FINISHED: u32 = 120;
const XP_PER_BEATEN: u32 = 30;
const XP_WON: u32 = 60;

/// The XP it takes to reach `level` from nothing. Level 1 is where everyone starts.
pub fn xp_for_level(level: u32) -> u32 {
    let steps = level.saturating_sub(1);
    // The sum of LEVEL_BASE + LEVEL_STEP * k for k in 0..steps.
    LEVEL_BASE * steps + LEVEL_STEP * steps * steps.saturating_sub(1) / 2
}

/// The level `xp` makes, from 1.
pub fn level(xp: u32) -> u32 {
    let mut level = 1;
    while xp_for_level(level + 1) <= xp {
        level += 1;
    }
    level
}

/// How far into its level `xp` is, from 0 to 1.
pub fn level_share(xp: u32) -> f32 {
    let at = level(xp);
    let (from, to) = (xp_for_level(at), xp_for_level(at + 1));
    (xp - from) as f32 / (to - from) as f32
}

/// The level that unlocks `reward`; 1 for what is free.
pub fn unlock_level(reward: Reward) -> u32 {
    REWARDS
        .iter()
        .find(|(_, unlocked)| *unlocked == reward)
        .map_or(1, |(level, _)| *level)
}

pub fn is_unlocked(reward: Reward, xp: u32) -> bool {
    level(xp) >= unlock_level(reward)
}

/// What became unlocked between `before` and `after` XP, in the order they were won.
pub fn unlocked_between(before: u32, after: u32) -> Vec<Reward> {
    let (from, to) = (level(before), level(after));
    REWARDS
        .iter()
        .filter(|(level, _)| *level > from && *level <= to)
        .map(|(_, reward)| *reward)
        .collect()
}

/// `look` with whatever `xp` has not unlocked yet put back to the first free one, and anything out
/// of range brought back in: what the server lets a player show.
pub fn allowed(look: CarLook, xp: u32) -> CarLook {
    let mut look = look.checked();
    if !is_unlocked(Reward::Car(look.model), xp) {
        look.model = 0;
    }
    if !is_unlocked(Reward::Trail(look.trail), xp) {
        look.trail = 0;
    }
    if !is_unlocked(Reward::Horn(look.horn), xp) {
        look.horn = 0;
    }
    look
}

/// The XP a race pays a player who finished at `place` (0 for the winner), or did not finish
/// (`None`), among `racers` who raced.
pub fn race_xp(place: Option<usize>, racers: usize) -> u32 {
    let Some(place) = place else {
        return XP_RACED;
    };
    let beaten = racers.saturating_sub(place + 1) as u32;
    let won = if place == 0 && racers > 1 { XP_WON } else { 0 };
    XP_RACED + XP_FINISHED + XP_PER_BEATEN * beaten + won
}

const _: () = assert!(CAR_MODELS == 2 * FREE && CAR_TRAILS == 2 * FREE && CAR_HORNS == 2 * FREE);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_grow_and_need_more_each_time() {
        assert_eq!(level(0), 1);
        assert_eq!(xp_for_level(1), 0);
        assert_eq!(xp_for_level(2), LEVEL_BASE);
        assert_eq!(xp_for_level(3), 2 * LEVEL_BASE + LEVEL_STEP);
        for xp in [0, 299, 300, 1000, 5000, 50_000] {
            let at = level(xp);
            assert!(xp_for_level(at) <= xp && xp < xp_for_level(at + 1), "{xp}");
            assert!((0.0..1.0).contains(&level_share(xp)));
        }
        assert_eq!(level(299), 1);
        assert_eq!(level(300), 2);
    }

    /// Everything to win is won, each once, the free ones never need winning, and the last car
    /// takes a few dozen races: about thirty solo races finished.
    #[test]
    fn every_reward_is_won_once_after_a_fair_number_of_races() {
        for kind in [Reward::Car, Reward::Trail, Reward::Horn] {
            for index in 0..FREE {
                assert_eq!(unlock_level(kind(index)), 1);
            }
            for index in FREE..2 * FREE {
                assert!(unlock_level(kind(index)) > 1);
                assert_eq!(REWARDS.iter().filter(|(_, r)| *r == kind(index)).count(), 1);
            }
        }
        assert!(REWARDS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        let solo = race_xp(Some(0), 1);
        let last = xp_for_level(REWARDS[REWARDS.len() - 1].0);
        let races = last.div_ceil(solo);
        assert!((20..=45).contains(&races), "{races} races");
    }

    #[test]
    fn a_locked_look_is_put_back_to_a_free_one() {
        let look = CarLook {
            model: 5,
            paint: 3,
            neon: 2,
            trail: 4,
            trail_color: 1,
            horn: 3,
        };
        let new = allowed(look, 0);
        assert_eq!((new.model, new.trail, new.horn), (0, 0, 0));
        assert_eq!((new.paint, new.neon, new.trail_color), (3, 2, 1));
        assert_eq!(allowed(look, 1_000_000), look);
        assert_eq!(
            unlocked_between(0, xp_for_level(4)),
            vec![Reward::Horn(3), Reward::Trail(3), Reward::Car(3)]
        );
        assert!(unlocked_between(xp_for_level(4), xp_for_level(4) + 1).is_empty());
    }

    #[test]
    fn finishing_ahead_pays_more() {
        assert!(race_xp(Some(0), 4) > race_xp(Some(1), 4));
        assert!(race_xp(Some(3), 4) > race_xp(None, 4));
        assert_eq!(race_xp(Some(0), 1), XP_RACED + XP_FINISHED);
    }
}
