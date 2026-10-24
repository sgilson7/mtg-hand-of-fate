//! Mana: colours, costs, and the bank a side spends from.
//!
//! A land on the map gives one unit of mana a turn. A unit can be one of a
//! set of colours: a basic land's unit is its colour, a guildgate's is
//! either of two, Chromatic Lantern makes every unit any colour. Unspent
//! units stay in the bank to the next turn (the person's rule: "generally
//! you bank mana turn to turn"), up to `BANK_CAP`.
//!
//! Paying a cost matches its symbols to units: coloured symbols first, the
//! most constrained first, each taking the unit with the fewest colours that
//! fits; generic symbols then take what is left, least flexible first.

use serde::{Deserialize, Serialize};

pub const W: u8 = 1;
pub const U: u8 = 2;
pub const B: u8 = 4;
pub const R: u8 = 8;
pub const G: u8 = 16;
/// Colourless: a unit only generic costs can use.
pub const C: u8 = 32;
pub const ANY: u8 = W | U | B | R | G;
pub const COLOURS: [u8; 5] = [W, U, B, R, G];

/// The most units a bank holds; income beyond it is lost.
pub const BANK_CAP: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sym {
    Generic(u8),
    Colour(u8),
    /// Either colour: {U/R}.
    Hybrid(u8, u8),
    /// X: chosen as the spell is cast.
    X,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    pub syms: Vec<Sym>,
}

pub fn colour_of(letter: char) -> Option<u8> {
    Some(match letter {
        'W' => W,
        'U' => U,
        'B' => B,
        'R' => R,
        'G' => G,
        'C' => C,
        _ => return None,
    })
}

pub fn letter_of(colour: u8) -> &'static str {
    match colour {
        W => "W",
        U => "U",
        B => "B",
        R => "R",
        G => "G",
        _ => "C",
    }
}

/// The colours a card's colour list names, as a mask.
pub fn mask_of(letters: &[String]) -> u8 {
    letters
        .iter()
        .filter_map(|l| l.chars().next().and_then(colour_of))
        .fold(0, |m, c| m | c)
}

pub fn count_colours(mask: u8) -> u32 {
    (mask & ANY).count_ones()
}

impl Cost {
    /// Parses "{2}{U}{U}{R}" and "{U/R}". An empty or missing cost is free.
    pub fn parse(text: &str) -> Result<Cost, String> {
        let mut syms = Vec::new();
        let mut rest = text.trim();
        while !rest.is_empty() {
            let open = rest
                .strip_prefix('{')
                .ok_or_else(|| format!("The mana cost \"{text}\" is not in braces."))?;
            let end = open
                .find('}')
                .ok_or_else(|| format!("The mana cost \"{text}\" has an open brace."))?;
            let tok = &open[..end];
            rest = &open[end + 1..];
            if tok == "X" {
                syms.push(Sym::X);
            } else if let Ok(n) = tok.parse::<u8>() {
                if n > 0 {
                    syms.push(Sym::Generic(n));
                }
            } else if let Some((a, b)) = tok.split_once('/') {
                let ca = a.chars().next().and_then(colour_of);
                let cb = b.chars().next().and_then(colour_of);
                match (ca, cb) {
                    (Some(x), Some(y)) => syms.push(Sym::Hybrid(x, y)),
                    _ => {
                        return Err(format!(
                            "The mana cost \"{text}\" has an unknown symbol {{{tok}}}."
                        ))
                    }
                }
            } else {
                let c = tok
                    .chars()
                    .next()
                    .and_then(colour_of)
                    .filter(|_| tok.len() == 1)
                    .ok_or_else(|| format!("The mana cost \"{text}\" has an unknown symbol {{{tok}}}."))?;
                syms.push(Sym::Colour(c));
            }
        }
        Ok(Cost { syms })
    }

    pub fn of(parts: &[Sym]) -> Cost {
        Cost { syms: parts.to_vec() }
    }

    /// The cost's mana value: what it would cost in units.
    pub fn value(&self) -> usize {
        self.syms
            .iter()
            .map(|s| match s {
                Sym::Generic(n) => *n as usize,
                Sym::X => 0,
                _ => 1,
            })
            .sum()
    }

    /// The same cost with up to `n` generic mana taken off.
    pub fn reduced(&self, mut n: u8) -> Cost {
        let mut syms = Vec::new();
        for s in &self.syms {
            match s {
                Sym::Generic(g) if n > 0 => {
                    let take = (*g).min(n);
                    n -= take;
                    if *g > take {
                        syms.push(Sym::Generic(g - take));
                    }
                }
                other => syms.push(*other),
            }
        }
        Cost { syms }
    }

    /// How many X the cost holds ({X}{X}{R}{R} holds two).
    pub fn xs(&self) -> u8 {
        self.syms.iter().filter(|s| matches!(s, Sym::X)).count() as u8
    }

    /// The cost with each X as `x` generic mana.
    pub fn with_x(&self, x: i32) -> Cost {
        let x = x.max(0) as u8;
        Cost {
            syms: self
                .syms
                .iter()
                .filter_map(|s| match s {
                    Sym::X if x == 0 => None,
                    Sym::X => Some(Sym::Generic(x)),
                    other => Some(*other),
                })
                .collect(),
        }
    }

    /// The colours its coloured and hybrid symbols ask for.
    pub fn colours(&self) -> u8 {
        self.syms.iter().fold(0, |m, s| match s {
            Sym::Colour(c) => m | c,
            Sym::Hybrid(a, b) => m | a | b,
            Sym::Generic(_) | Sym::X => m,
        })
    }
}

/// A side's mana: one entry a unit, each the colours it can be.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bank {
    pub units: Vec<u8>,
}

impl Bank {
    pub fn add(&mut self, unit: u8) {
        if self.units.len() < BANK_CAP {
            self.units.push(unit);
        }
    }

    pub fn len(&self) -> usize {
        self.units.len()
    }

    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// Makes every coloured unit any colour (Chromatic Lantern).
    pub fn any_colour(&mut self) {
        self.units.fill(ANY);
    }

    /// Which units would pay `cost`, by index, or None if the bank cannot.
    pub fn plan(&self, cost: &Cost) -> Option<Vec<usize>> {
        let mut used = vec![false; self.units.len()];
        let mut picks = Vec::new();
        // coloured needs, the most constrained (fewest acceptable colours) first
        let mut needs: Vec<u8> = cost
            .syms
            .iter()
            .filter_map(|s| match s {
                Sym::Colour(c) => Some(*c),
                Sym::Hybrid(a, b) => Some(a | b),
                Sym::Generic(_) | Sym::X => None,
            })
            .collect();
        needs.sort_by_key(|m| count_colours(*m));
        // each need tries the least flexible unit first, and backs off when a
        // later need is left with nothing (two gates sharing a colour)
        if !self.assign(&needs, &mut used, &mut picks) {
            return None;
        }
        let generic: usize = cost
            .syms
            .iter()
            .map(|s| match s {
                Sym::Generic(n) => *n as usize,
                _ => 0,
            })
            .sum();
        for _ in 0..generic {
            let best = (0..self.units.len()).filter(|&i| !used[i]).min_by_key(|&i| {
                (
                    if self.units[i] == C {
                        0
                    } else {
                        1 + count_colours(self.units[i])
                    },
                    i,
                )
            })?;
            used[best] = true;
            picks.push(best);
        }
        Some(picks)
    }

    fn assign(&self, needs: &[u8], used: &mut [bool], picks: &mut Vec<usize>) -> bool {
        let Some((&need, rest)) = needs.split_first() else {
            return true;
        };
        let mut options: Vec<usize> = (0..self.units.len())
            .filter(|&i| !used[i] && self.units[i] & need != 0)
            .collect();
        options.sort_by_key(|&i| (count_colours(self.units[i]), i));
        for i in options {
            used[i] = true;
            picks.push(i);
            if self.assign(rest, used, picks) {
                return true;
            }
            picks.pop();
            used[i] = false;
        }
        false
    }

    pub fn can_pay(&self, cost: &Cost) -> bool {
        self.plan(cost).is_some()
    }

    /// Pays `cost`, or returns false and leaves the bank as it was.
    pub fn pay(&mut self, cost: &Cost) -> bool {
        let Some(mut picks) = self.plan(cost) else {
            return false;
        };
        picks.sort_unstable();
        for i in picks.into_iter().rev() {
            self.units.remove(i);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cost_parses_generic_coloured_and_hybrid_symbols() {
        let c = Cost::parse("{2}{U}{U}{R}{R}").unwrap();
        assert_eq!(c.value(), 6);
        assert_eq!(c.syms[0], Sym::Generic(2));
        let h = Cost::parse("{U/R}").unwrap();
        assert_eq!(h.syms, vec![Sym::Hybrid(U, R)]);
        assert_eq!(Cost::parse("").unwrap().value(), 0);
        assert!(Cost::parse("{Q}").is_err());
    }

    #[test]
    fn a_guildgate_unit_pays_either_colour_but_only_once() {
        let mut bank = Bank { units: vec![U | R] };
        assert!(bank.can_pay(&Cost::parse("{R}").unwrap()));
        assert!(!bank.can_pay(&Cost::parse("{U}{R}").unwrap()));
        assert!(bank.pay(&Cost::parse("{U}").unwrap()));
        assert!(bank.is_empty());
    }

    #[test]
    fn paying_saves_the_flexible_unit_for_the_colour_only_it_can_pay() {
        // {R}{U}: the Mountain must pay R, so the guildgate pays U
        let bank = Bank {
            units: vec![U | R, R],
        };
        let picks = bank.plan(&Cost::parse("{R}{U}").unwrap()).unwrap();
        assert_eq!(picks.len(), 2);
        // generic takes the least flexible unit: the Swamp, not the gate
        let bank = Bank {
            units: vec![U | R, B],
        };
        assert_eq!(bank.plan(&Cost::parse("{1}").unwrap()).unwrap(), vec![1]);
    }

    #[test]
    fn two_gates_sharing_a_colour_still_pay_both_colours() {
        // Azorius (W/U) first, Selesnya (G/W), Mountain: {1}{W}{U} is payable
        // as Selesnya for W, Azorius for U, Mountain for the 1
        let bank = Bank {
            units: vec![W | U, G | W, R],
        };
        let picks = bank.plan(&Cost::parse("{1}{W}{U}").unwrap()).expect("payable");
        assert_eq!(picks.len(), 3);
        let bank = Bank {
            units: vec![G | W, B | G],
        };
        assert!(bank.can_pay(&Cost::parse("{G}{W}").unwrap()));
    }

    #[test]
    fn a_failed_payment_leaves_the_bank_alone() {
        let mut bank = Bank { units: vec![G, G] };
        assert!(!bank.pay(&Cost::parse("{1}{W}").unwrap()));
        assert_eq!(bank.units, vec![G, G]);
    }

    #[test]
    fn the_bank_stops_at_its_cap() {
        let mut bank = Bank::default();
        for _ in 0..BANK_CAP + 3 {
            bank.add(R);
        }
        assert_eq!(bank.len(), 12);
    }

    #[test]
    fn a_reduction_takes_generic_mana_only() {
        let c = Cost::parse("{1}{R}").unwrap().reduced(1);
        assert_eq!(c.syms, vec![Sym::Colour(R)]);
        let c = Cost::parse("{R}").unwrap().reduced(1);
        assert_eq!(c.syms, vec![Sym::Colour(R)]);
    }
}
