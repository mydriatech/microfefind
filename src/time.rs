/*
    Copyright 2024 MydriaTech AB

    Licensed under the Apache License 2.0 with Free world makers exception
    1.0.0 (the "License"); you may not use this file except in compliance with
    the License. You should have obtained a copy of the License with the source
    or binary distribution in file named

        LICENSE-Apache-2.0-with-FWM-Exception-1.0.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

//! Time utilities.

use std::time::{Duration, SystemTime};

/// Return elapsed milliseconds since Unix Epoch time.
pub fn now_as_millis() -> u64 {
    u64::try_from(now().as_millis()).unwrap()
}

/// Return elapsed seconds since Unix Epoch time.
pub fn now_as_secs() -> u64 {
    now().as_secs()
}

/// Return [Duration] since Unix Epoch time.
fn now() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}
