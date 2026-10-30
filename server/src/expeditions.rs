use rand::{rngs::StdRng, Rng, SeedableRng};

#[derive(Clone, Debug)]
pub struct RewardRule {
    pub item_id: i64,
    pub item_slug: String,
    pub item_name: String,
    pub min_quantity: i32,
    pub max_quantity: i32,
    pub weight: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewardOutcome {
    pub item_id: i64,
    pub item_slug: String,
    pub item_name: String,
    pub quantity: i32,
}

pub fn roll_reward(seed: i64, rules: &[RewardRule]) -> Option<RewardOutcome> {
    if rules.is_empty()
        || rules.iter().any(|rule| {
            rule.weight <= 0 || rule.min_quantity <= 0 || rule.max_quantity < rule.min_quantity
        })
    {
        return None;
    }

    let total_weight = rules
        .iter()
        .try_fold(0_u64, |sum, rule| sum.checked_add(rule.weight as u64))?;
    let mut rng = StdRng::seed_from_u64(seed as u64);
    let mut selection = rng.gen_range(0..total_weight);
    let selected = rules.iter().find(|rule| {
        if selection < rule.weight as u64 {
            true
        } else {
            selection -= rule.weight as u64;
            false
        }
    })?;
    let quantity = rng.gen_range(selected.min_quantity..=selected.max_quantity);

    Some(RewardOutcome {
        item_id: selected.item_id,
        item_slug: selected.item_slug.clone(),
        item_name: selected.item_name.clone(),
        quantity,
    })
}

#[cfg(test)]
mod tests {
    use super::{roll_reward, RewardRule};

    fn rules() -> Vec<RewardRule> {
        vec![
            RewardRule {
                item_id: 1,
                item_slug: "echo_fragment".to_owned(),
                item_name: "Fragmento de eco".to_owned(),
                min_quantity: 1,
                max_quantity: 3,
                weight: 3,
            },
            RewardRule {
                item_id: 2,
                item_slug: "abyssal_ore".to_owned(),
                item_name: "Mineral abisal".to_owned(),
                min_quantity: 1,
                max_quantity: 2,
                weight: 1,
            },
        ]
    }

    #[test]
    fn same_seed_and_rules_reproduce_the_same_reward() {
        assert_eq!(roll_reward(42, &rules()), roll_reward(42, &rules()));
    }

    #[test]
    fn invalid_reward_rules_are_rejected() {
        let mut invalid_rules = rules();
        invalid_rules[0].weight = 0;
        assert_eq!(roll_reward(42, &invalid_rules), None);
    }
}
