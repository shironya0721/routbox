use std::collections::{HashMap, HashSet};

use crate::{
    config::{KeyMappingConfig, KeyTriggerTiming},
    event::InputEvent,
    key_sender::TourAction,
};

#[derive(Debug)]
pub struct KeyMappingEntry {
    trigger_key: String,
    action: String,
    modifier: Vec<String>,
    trigger: KeyTriggerTiming,
}

pub struct KeyMappingProcessor {
    // as the entrys won't change after it is created, usize is pointing to entrys
    entrys: Vec<KeyMappingEntry>,
    // config with mappings
    mappings: HashMap<String, Vec<usize>>,
    // store pressed_key of tourbox
    pressed_key: HashSet<String>,
    // outputed action
    output_action: Vec<usize>,
}

impl KeyMappingProcessor {
    fn get_actived_action(&self, ev: &InputEvent) -> Option<usize> {
        // v.modifier key should not be possible more than 1000
        let delta = match ev {
            InputEvent::KeyPressed(_) => 1000,
            InputEvent::KeyReleased(_) => -1000,
        };

        let k = match ev {
            InputEvent::KeyPressed(k) => k,
            InputEvent::KeyReleased(k) => k,
        };

        if let Some(key_mapping) = self.mappings.get(k) {
            key_mapping
                .iter()
                .filter_map(|kk| {
                    let k = &self.entrys[*kk];
                    if k.modifier.iter().all(|k| self.pressed_key.contains(k)) {
                        Some(*kk)
                    } else {
                        None
                    }
                })
                .max_by_key(|kk| {
                    let k = &self.entrys[*kk];

                    k.modifier.len() as i32
                        + match k.trigger {
                            KeyTriggerTiming::OnPress => delta,
                            KeyTriggerTiming::OnHold => 1000,
                            KeyTriggerTiming::OnRelease => -delta,
                        }
                })
        } else {
            None
        }
    }

    pub fn process(&mut self, ev: InputEvent) -> Vec<TourAction> {
        println!("+{:?}", ev);
        let actived_key_index = self.get_actived_action(&ev);
        let actived_key = actived_key_index.as_ref().map(|k| &self.entrys[*k]);

        let mut key_actions = vec![];

        match ev {
            InputEvent::KeyPressed(k) => {
                if let Some(actived_key) = actived_key {
                    match &actived_key.trigger {
                        KeyTriggerTiming::OnPress => {
                            println!("Action {}", actived_key.action);
                            key_actions.push(TourAction::KeyClick(actived_key.action.clone()));
                        }
                        KeyTriggerTiming::OnHold => {
                            let new_output_key: Vec<_> = actived_key.action.split("+").collect();

                            let mut new_output_action: Vec<usize> = self
                                .output_action
                                .iter()
                                .filter_map(|vk| {
                                    let v = &self.entrys[*vk];
                                    let b = actived_key
                                        .modifier
                                        .iter()
                                        .any(|mv| v.modifier.contains(mv) || &v.trigger_key == mv);

                                    if b {
                                        for kb in v.action.split("+") {
                                            // if we won't add back the key at new action (new_output_key), then release the key
                                            if !new_output_key.contains(&kb) {
                                                key_actions
                                                    .push(TourAction::KeyRelease(kb.to_owned()));
                                            }
                                        }
                                        None
                                    } else {
                                        Some(*vk)
                                    }
                                })
                                .collect();

                            for kb in new_output_key {
                                // it is assumed that press a pressed key is fine
                                key_actions.push(TourAction::KeyPress(kb.to_owned()));
                            }

                            new_output_action.push(actived_key_index.unwrap());

                            drop(std::mem::replace(
                                &mut self.output_action,
                                new_output_action,
                            ));
                        }
                        KeyTriggerTiming::OnRelease => {
                            // do nothing on release
                        }
                    }
                }
                self.pressed_key.insert(k);
            }
            InputEvent::KeyReleased(k) => {
                if let Some(actived_key) = actived_key {
                    match &actived_key.trigger {
                        KeyTriggerTiming::OnRelease => {
                            println!("Action {}", actived_key.action);
                            key_actions.push(TourAction::KeyClick(actived_key.action.clone()));
                        }
                        _ => {
                            // do nothing
                        }
                    }
                }

                let new_hold_action: Vec<usize> = self
                    .output_action
                    .iter()
                    .filter_map(|vk| {
                        let v = &self.entrys[*vk];
                        if v.trigger_key == k || v.modifier.iter().any(|mk| mk == &k) {
                            // release hold action releated key when release the input key
                            for kb in v.action.split("+") {
                                key_actions.push(TourAction::KeyRelease(kb.to_owned()));
                            }
                            None
                        } else {
                            Some(*vk)
                        }
                    })
                    .collect();

                drop(std::mem::replace(&mut self.output_action, new_hold_action));
                self.pressed_key.remove(&k);
            }
        }

        key_actions
    }

    pub fn from_config(mappings: &Vec<KeyMappingConfig>) -> Self {
        let mut trigger_key_map = HashMap::new();
        let mut entrys = vec![];
        mappings.iter().for_each(|m| {
            let mut key_iter = m.keys.split("+");
            let mut modifiers = vec![];
            let mut trigger_key = key_iter
                .next()
                .expect("Should be at least contains one key")
                .to_owned();
            while let Some(k) = key_iter.next() {
                modifiers.push(std::mem::replace(&mut trigger_key, k.to_owned()));
            }
            if !trigger_key_map.contains_key(&trigger_key) {
                trigger_key_map.insert(trigger_key.clone(), vec![]);
            }

            trigger_key_map
                .get_mut(&trigger_key)
                .unwrap()
                .push(entrys.len());

            entrys.push(KeyMappingEntry {
                trigger_key,
                action: m.action.clone(),
                modifier: modifiers,
                trigger: m.trigger,
            });
        });

        Self {
            entrys,
            mappings: trigger_key_map,
            pressed_key: HashSet::new(),
            output_action: vec![],
        }
    }
}
