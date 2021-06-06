use crate::server;

///Events that can be injected into the state machine
pub enum SMEvents {
    IfStatusChanged(bool),
    Configured(bool),
}

#[derive(Debug)]
pub enum SDServerStateMachine {
    NotReady(SDServerStateMachineData<NotReady>),
    Ready(SDServerStateMachineData<ReadyStateWrapper>),
}

impl Default for SDServerStateMachine {
    fn default() -> Self {
        SDServerStateMachine::NotReady(SDServerStateMachineData::<NotReady>::default())
    }
}

//impl From<SDServerStateMachine>

impl SDServerStateMachine {
    pub fn next(self, event: SMEvents) -> Self {
        match (self, event) {
            (SDServerStateMachine::NotReady(mut from), SMEvents::IfStatusChanged(enabled)) => {
                from.inner.up_and_configured = enabled;
                if from.inner.up_and_configured && from.inner.service_status {
                    SDServerStateMachine::Ready(SDServerStateMachineData::<ReadyStateWrapper> {
                        state: ReadyStateWrapper::default(),
                        inner: from.inner,
                    })
                } else {
                    // no transition,
                    SDServerStateMachine::NotReady(from)
                }
            }
            (SDServerStateMachine::NotReady(mut from), SMEvents::Configured(service_status)) => {
                from.inner.service_status = service_status;
                if from.inner.up_and_configured && from.inner.service_status {
                    SDServerStateMachine::Ready(SDServerStateMachineData::<ReadyStateWrapper> {
                        state: ReadyStateWrapper::default(),
                        inner: from.inner,
                    })
                } else {
                    // no transition,
                    SDServerStateMachine::NotReady(from)
                }
            }
            (SDServerStateMachine::Ready(mut from), SMEvents::IfStatusChanged(enabled)) => {
                todo!()
            }
            (SDServerStateMachine::Ready(mut from), SMEvents::Configured(service_status)) => {
                todo!()
            }
        }
    }
}

#[derive(Debug)]
pub struct SDServerStateMachineData<S> {
    state: S,
    inner: SDServerStateMachineInnerData,
}

#[derive(Debug, Default)]
struct SDServerStateMachineInnerData {
    pub up_and_configured: bool,
    pub service_status: bool,
}

impl<S> Default for SDServerStateMachineData<S>
where
    S: Default,
{
    fn default() -> Self {
        SDServerStateMachineData {
            state: S::default(),
            inner: SDServerStateMachineInnerData::default(),
        }
    }
}

#[derive(Default, Debug)]
pub struct NotReady {}

#[derive(Debug)]
pub enum ReadyStateWrapper {
    InitialWaitPhase(Ready<ReadyInitialWaitPhase>),
    RepetitionPhase(Ready<ReadyRepetitionPhase>),
    MainPhase(Ready<ReadyMainPhase>),
}

impl Default for ReadyStateWrapper {
    fn default() -> Self {
        ReadyStateWrapper::InitialWaitPhase(Ready::<ReadyInitialWaitPhase>::default())
    }
}

#[derive(Debug)]
pub struct Ready<S> {
    state: S,
}

impl<S> Default for Ready<S>
where
    S: Default,
{
    fn default() -> Self {
        Ready {
            state: S::default(),
        }
    }
}

#[derive(Default, Debug)]
pub struct ReadyInitialWaitPhase {}
#[derive(Default, Debug)]
pub struct ReadyRepetitionPhase {}
#[derive(Default, Debug)]
pub struct ReadyMainPhase {}

//Transitions

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ctor() {
        let server_sm = SDServerStateMachine::default();
        println!("Server State Machine wrapper: {:?}", &server_sm);
        let server_sm = server_sm.next(SMEvents::Configured(true));
        println!("Server State Machine wrapper: {:?}", &server_sm);
        let server_sm = server_sm.next(SMEvents::IfStatusChanged(true));
        println!("Server State Machine wrapper: {:?}", &server_sm);
    }
}
