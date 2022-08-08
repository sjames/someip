/*
    Copyright 2021 Sojan James
    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at
        http://www.apache.org/licenses/LICENSE-2.0
    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

use rand::prelude::*;

///Events that can be injected into the state machine

pub trait State {
    fn on_entry(&mut self) {}
    fn on_exit(&mut self) {}
    // The state implements this if it is a sub-state machine. The default handler just returns the current state.
    fn next(self, _event: SMEvent) -> Self
    where
        Self: Sized,
    {
        self
    }
}

pub enum SMEvent {
    IfStatusChanged(bool),
    ServiceConfiguration(bool),
    Timeout(u16, u16),
    FindService,
}

struct SMData {
    interface_status: bool,
    service_status: bool,
    service_id: u16,
    set_timer: Box<dyn FnMut(u16, std::time::Duration)>,
    clear_timer: Box<dyn FnMut(u16, u16)>,
    send_offer: Box<dyn FnMut()>,
    initial_delay_min: std::time::Duration,
    initial_delay_max: std::time::Duration,
    repetitions_max: u32,
    repetitions_base_delay: std::time::Duration,
    cyclic_announce_delay: std::time::Duration,
}

impl SMData {
    fn get_initial_delay(&self) -> std::time::Duration {
        let mut rng = rand::thread_rng();
        let delay =
            rng.gen_range(self.initial_delay_min.as_millis()..self.initial_delay_max.as_millis());
        std::time::Duration::from_millis(delay as u64)
    }
}

pub struct Data<S> {
    state: S,
    inner: SMData,
}

pub struct SDServerStateMachineContainer {
    state: Option<SDServerStateMachine>,
}

impl SDServerStateMachineContainer {
    pub fn new(
        service_id: u16,
        set_timer: Box<dyn FnMut(u16, std::time::Duration)>,
        clear_timer: Box<dyn FnMut(u16, u16)>,
        send_offer: Box<dyn FnMut()>,
        initial_delay_min: std::time::Duration,
        initial_delay_max: std::time::Duration,
        repetitions_max: u32,
        repetitions_base_delay: std::time::Duration,
        cyclic_announce_delay: std::time::Duration,
    ) -> Self {
        Self {
            state: Some(SDServerStateMachine::new(
                service_id,
                set_timer,
                clear_timer,
                send_offer,
                initial_delay_min,
                initial_delay_max,
                repetitions_max,
                repetitions_base_delay,
                cyclic_announce_delay,
            )),
        }
    }

    pub fn next(&mut self, event: SMEvent) {
        let state = self.state.take().unwrap();
        self.state = Some(state.next(event));
    }
}
pub enum SDServerStateMachine {
    NotReady(Data<NotReady>),
    InitialWaitPhase(Data<InitialWaitPhase>),
    RepetitionPhase(Data<RepetitionPhase>),
    MainPhase(Data<MainPhase>),
}

impl SDServerStateMachine {
    pub fn new(
        service_id: u16,
        set_timer: Box<dyn FnMut(u16, std::time::Duration)>,
        clear_timer: Box<dyn FnMut(u16, u16)>,
        send_offer: Box<dyn FnMut()>,
        initial_delay_min: std::time::Duration,
        initial_delay_max: std::time::Duration,
        repetitions_max: u32,
        repetitions_base_delay: std::time::Duration,
        cyclic_announce_delay: std::time::Duration,
    ) -> Self {
        SDServerStateMachine::NotReady(Data::<NotReady> {
            state: NotReady::default(),
            inner: SMData {
                interface_status: false,
                service_status: false,
                service_id,
                set_timer,
                clear_timer,
                send_offer,
                initial_delay_min,
                initial_delay_max,
                repetitions_max,
                repetitions_base_delay,
                cyclic_announce_delay,
            },
        })
    }
}

impl From<Data<NotReady>> for Data<InitialWaitPhase> {
    fn from(mut d: Data<NotReady>) -> Self {
        d.state.on_exit();
        let mut state = InitialWaitPhase::default();
        state.on_entry();
        Self {
            inner: d.inner,
            state,
        }
    }
}

impl From<Data<InitialWaitPhase>> for Data<RepetitionPhase> {
    fn from(mut d: Data<InitialWaitPhase>) -> Self {
        d.state.on_exit();
        let mut state = RepetitionPhase::default();
        state.on_entry();
        Self {
            inner: d.inner,
            state,
        }
    }
}

impl From<Data<InitialWaitPhase>> for Data<NotReady> {
    fn from(mut d: Data<InitialWaitPhase>) -> Self {
        d.state.on_exit();
        let mut state = NotReady::default();
        state.on_entry();
        Self {
            inner: d.inner,
            state,
        }
    }
}

impl From<Data<RepetitionPhase>> for Data<NotReady> {
    fn from(mut d: Data<RepetitionPhase>) -> Self {
        d.state.on_exit();
        let mut state = NotReady::default();
        state.on_entry();
        Self {
            inner: d.inner,
            state,
        }
    }
}

impl From<Data<MainPhase>> for Data<NotReady> {
    fn from(mut d: Data<MainPhase>) -> Self {
        d.state.on_exit();
        let mut state = NotReady::default();
        state.on_entry();
        Self {
            inner: d.inner,
            state,
        }
    }
}

impl From<Data<RepetitionPhase>> for Data<MainPhase> {
    fn from(mut d: Data<RepetitionPhase>) -> Self {
        d.state.on_exit();
        let mut state = MainPhase::default();
        state.on_entry();
        Self {
            inner: d.inner,
            state,
        }
    }
}

impl State for SDServerStateMachine {
    fn next(self, event: SMEvent) -> Self
    where
        Self: Sized,
    {
        match (self, event) {
            (SDServerStateMachine::NotReady(mut d), SMEvent::IfStatusChanged(if_enabled)) => {
                d.inner.interface_status = if_enabled;
                if d.inner.interface_status && d.inner.service_status {
                    let delay = d.inner.get_initial_delay();
                    (d.inner.set_timer)(0, delay);
                    SDServerStateMachine::InitialWaitPhase(d.into())
                } else {
                    SDServerStateMachine::NotReady(d)
                }
            }
            (SDServerStateMachine::NotReady(mut d), SMEvent::ServiceConfiguration(enabled)) => {
                d.inner.service_status = enabled;
                if d.inner.interface_status && d.inner.service_status {
                    let delay = d.inner.get_initial_delay();
                    (d.inner.set_timer)(0, delay);
                    SDServerStateMachine::InitialWaitPhase(d.into())
                } else {
                    SDServerStateMachine::NotReady(d)
                }
            }
            (SDServerStateMachine::NotReady(_d), SMEvent::Timeout(_, _)) => {
                panic!("This should not happen")
            }
            // don't do anything if not ready
            (SDServerStateMachine::NotReady(d), SMEvent::FindService) => {
                SDServerStateMachine::NotReady(d)
            }

            // Go back to not-ready of interface is disabled
            (
                SDServerStateMachine::InitialWaitPhase(mut d),
                SMEvent::IfStatusChanged(if_enabled),
            ) => {
                d.inner.interface_status = if_enabled;
                if d.inner.interface_status && d.inner.service_status {
                    SDServerStateMachine::InitialWaitPhase(d)
                } else {
                    (d.inner.clear_timer)(0, d.inner.service_id);
                    SDServerStateMachine::NotReady(d.into())
                }
            }
            // go back to not-ready if service is disabled
            (
                SDServerStateMachine::InitialWaitPhase(mut d),
                SMEvent::ServiceConfiguration(enabled),
            ) => {
                d.inner.service_status = enabled;
                if d.inner.interface_status && d.inner.service_status {
                    SDServerStateMachine::InitialWaitPhase(d)
                } else {
                    (d.inner.clear_timer)(0, d.inner.service_id);
                    SDServerStateMachine::NotReady(d.into())
                }
            }
            //Timeout in Initial wait phase. Timer id should be 0
            (SDServerStateMachine::InitialWaitPhase(mut d), SMEvent::Timeout(tid, _service_id)) => {
                if tid == 0 {
                    //todo: SendOffer service
                    (d.inner.set_timer)(1, d.inner.repetitions_base_delay);
                    SDServerStateMachine::RepetitionPhase(d.into())
                } else {
                    println!("Unexpected timer id for timeout:{}", tid);
                    SDServerStateMachine::InitialWaitPhase(d)
                }
            }
            // do nothing here. Ignore find service in the initial wait phase.
            (SDServerStateMachine::InitialWaitPhase(d), SMEvent::FindService) => {
                SDServerStateMachine::InitialWaitPhase(d)
            }
            // interface disable, go back to not ready
            (
                SDServerStateMachine::RepetitionPhase(mut d),
                SMEvent::IfStatusChanged(if_enabled),
            ) => {
                d.inner.interface_status = if_enabled;
                if !if_enabled {
                    SDServerStateMachine::NotReady(d.into())
                } else {
                    SDServerStateMachine::RepetitionPhase(d)
                }
            }
            (
                SDServerStateMachine::RepetitionPhase(mut d),
                SMEvent::ServiceConfiguration(enabled),
            ) => {
                d.inner.service_status = enabled;
                if !enabled {
                    SDServerStateMachine::NotReady(d.into())
                } else {
                    SDServerStateMachine::RepetitionPhase(d)
                }
            }
            (SDServerStateMachine::RepetitionPhase(mut d), SMEvent::Timeout(tid, _service_id)) => {
                (d.inner.send_offer)();
                d.state.run += 1;
                assert!(tid == 1);
                if d.state.run > d.inner.repetitions_max {
                    (d.inner.set_timer)(2, d.inner.cyclic_announce_delay);
                    SDServerStateMachine::MainPhase(d.into())
                } else {
                    (d.inner.set_timer)(1, d.inner.repetitions_base_delay);
                    SDServerStateMachine::RepetitionPhase(d)
                }
            }
            (SDServerStateMachine::RepetitionPhase(mut d), SMEvent::FindService) => {
                (d.inner.clear_timer)(1, d.inner.service_id); // reset the timer
                (d.inner.send_offer)();
                (d.inner.set_timer)(1, d.inner.repetitions_base_delay);
                SDServerStateMachine::RepetitionPhase(d)
            }
            (SDServerStateMachine::MainPhase(mut d), SMEvent::IfStatusChanged(if_enabled)) => {
                d.inner.interface_status = if_enabled;
                if !if_enabled {
                    SDServerStateMachine::NotReady(d.into())
                } else {
                    SDServerStateMachine::MainPhase(d)
                }
            }
            (SDServerStateMachine::MainPhase(mut d), SMEvent::ServiceConfiguration(enabled)) => {
                d.inner.service_status = enabled;
                if !enabled {
                    SDServerStateMachine::NotReady(d.into())
                } else {
                    SDServerStateMachine::MainPhase(d)
                }
            }
            (SDServerStateMachine::MainPhase(mut d), SMEvent::Timeout(tid, _service_id)) => {
                assert!(tid == 2);
                (d.inner.send_offer)();
                (d.inner.set_timer)(2, d.inner.cyclic_announce_delay);
                SDServerStateMachine::MainPhase(d)
            }
            (SDServerStateMachine::MainPhase(mut d), SMEvent::FindService) => {
                (d.inner.clear_timer)(2, d.inner.service_id); // reset the timer
                (d.inner.send_offer)();
                (d.inner.set_timer)(2, d.inner.repetitions_base_delay);
                SDServerStateMachine::MainPhase(d)
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct NotReady {}
impl State for NotReady {}

#[derive(Default, Debug)]
pub struct InitialWaitPhase {}
impl State for InitialWaitPhase {}
#[derive(Default, Debug)]
pub struct RepetitionPhase {
    run: u32,
}
impl State for RepetitionPhase {}
#[derive(Default, Debug)]
pub struct MainPhase {}
impl State for MainPhase {}

#[cfg(test)]
mod test {
    use super::*;

    fn create_ready_sm() -> SDServerStateMachineContainer {
        let mut server_sm = SDServerStateMachineContainer::new(
            0x42,
            Box::new(|timer_id, duration| {
                println!("Setting timer {} for {:?}", timer_id, duration);
            }),
            Box::new(|timer_id, _service_id| {
                println!("Resetting timer {} ", timer_id);
            }),
            Box::new(|| {
                println!("Send Offer");
            }),
            std::time::Duration::from_millis(10),
            std::time::Duration::from_millis(100),
            3,
            std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(5),
        );
        assert!(matches!(
            server_sm.state,
            Some(SDServerStateMachine::NotReady(_))
        ));
        server_sm.next(SMEvent::ServiceConfiguration(true));
        assert!(matches!(
            server_sm.state,
            Some(SDServerStateMachine::NotReady(_))
        ));
        server_sm.next(SMEvent::IfStatusChanged(true));
        assert!(matches!(
            server_sm.state,
            Some(SDServerStateMachine::InitialWaitPhase(_))
        ));
        server_sm.next(SMEvent::IfStatusChanged(false));
        assert!(matches!(
            server_sm.state,
            Some(SDServerStateMachine::NotReady(_))
        ));
        server_sm.next(SMEvent::IfStatusChanged(true));
        assert!(matches!(
            server_sm.state,
            Some(SDServerStateMachine::InitialWaitPhase(_))
        ));
        server_sm
    }

    #[test]
    fn test_happy_path() {
        let mut sm = create_ready_sm();
        sm.next(SMEvent::Timeout(0, 42));
        assert!(matches!(
            sm.state,
            Some(SDServerStateMachine::RepetitionPhase(_))
        ));
        sm.next(SMEvent::Timeout(1, 42));
        assert!(matches!(
            sm.state,
            Some(SDServerStateMachine::RepetitionPhase(_))
        ));
        sm.next(SMEvent::Timeout(1, 42));
        sm.next(SMEvent::Timeout(1, 42));
        sm.next(SMEvent::Timeout(1, 42));
        assert!(matches!(sm.state, Some(SDServerStateMachine::MainPhase(_))));
        sm.next(SMEvent::Timeout(2, 42));
        assert!(matches!(sm.state, Some(SDServerStateMachine::MainPhase(_))));
        sm.next(SMEvent::IfStatusChanged(false));
        assert!(matches!(sm.state, Some(SDServerStateMachine::NotReady(_))));
    }
}
