use crate::server;
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
    Timeout(u32),
    FindService,
}

struct SMData {
    interface_status: bool,
    service_status: bool,
    set_timer: Box<dyn FnMut(u32, std::time::Duration)>,
    reset_timer: Box<dyn FnMut(u32)>,
    initial_delay_min: std::time::Duration,
    initial_delay_max: std::time::Duration,
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

pub enum SDServerStateMachine {
    NotReady(Data<NotReady>),
    InitialWaitPhase(Data<InitialWaitPhase>),
    RepetitionPhase(Data<RepetitionPhase>),
    MainPhase(Data<MainPhase>),
}

impl SDServerStateMachine {
    pub fn new(
        set_timer: Box<dyn FnMut(u32, std::time::Duration)>,
        reset_timer: Box<dyn FnMut(u32)>,
        initial_delay_min: std::time::Duration,
        initial_delay_max: std::time::Duration,
    ) -> Self {
        SDServerStateMachine::NotReady(Data::<NotReady> {
            state: NotReady::default(),
            inner: SMData {
                interface_status: false,
                service_status: false,
                set_timer,
                reset_timer,
                initial_delay_min,
                initial_delay_max,
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

impl State for SDServerStateMachine {
    fn next(mut self, event: SMEvent) -> Self
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
            (SDServerStateMachine::NotReady(d), SMEvent::Timeout(_)) => {
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
                    SDServerStateMachine::NotReady(d.into())
                }
            }
            //Timeout in Initial wait phase. Timer id should be 0
            (SDServerStateMachine::InitialWaitPhase(d), SMEvent::Timeout(tid)) => {
                if tid == 0 {
                    //todo: SendOffer service
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
            (SDServerStateMachine::RepetitionPhase(_), SMEvent::ServiceConfiguration(_)) => todo!(),
            (SDServerStateMachine::RepetitionPhase(_), SMEvent::Timeout(_)) => todo!(),
            (SDServerStateMachine::RepetitionPhase(_), SMEvent::FindService) => todo!(),
            (SDServerStateMachine::MainPhase(_), SMEvent::IfStatusChanged(_)) => todo!(),
            (SDServerStateMachine::MainPhase(_), SMEvent::ServiceConfiguration(_)) => todo!(),
            (SDServerStateMachine::MainPhase(_), SMEvent::Timeout(_)) => todo!(),
            (SDServerStateMachine::MainPhase(_), SMEvent::FindService) => todo!(),
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
pub struct RepetitionPhase {}
impl State for RepetitionPhase {}
#[derive(Default, Debug)]
pub struct MainPhase {}

#[cfg(test)]
mod test {
    use super::*;

    fn create_ready_sm() -> SDServerStateMachine {
        let server_sm = SDServerStateMachine::new(
            Box::new(|timer_id, duration| {
                println!("Setting timer {} for {:?}", timer_id, duration);
            }),
            Box::new(|timer_id| {
                println!("Resetting timer {} ", timer_id);
            }),
            std::time::Duration::from_millis(10),
            std::time::Duration::from_millis(100),
        );
        assert!(matches!(server_sm, SDServerStateMachine::NotReady(_)));
        let server_sm = server_sm.next(SMEvent::ServiceConfiguration(true));
        assert!(matches!(server_sm, SDServerStateMachine::NotReady(_)));
        let server_sm = server_sm.next(SMEvent::IfStatusChanged(true));
        assert!(matches!(
            server_sm,
            SDServerStateMachine::InitialWaitPhase(_)
        ));
        let server_sm = server_sm.next(SMEvent::IfStatusChanged(false));
        assert!(matches!(server_sm, SDServerStateMachine::NotReady(_)));
        server_sm
    }

    #[test]
    fn test_ctor() {
        let sm = create_ready_sm();
    }
}
