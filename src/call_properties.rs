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
pub struct CallProperties {
    pub timeout: std::time::Duration,
}

impl Default for CallProperties {
    fn default() -> Self {
        CallProperties {
            timeout: std::time::Duration::from_secs(1),
        }
    }
}

impl CallProperties {
    pub fn with_timeout(timeout: std::time::Duration) -> Self {
        Self {
            timeout,
            //..Default::default()
        }
    }
}
