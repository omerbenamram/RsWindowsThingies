use std::ptr::null_mut;
use std::ffi::OsString;
use std::os::windows::prelude::*;
use serde_json::Value;
use winapi::um::winevt::*;
use winapi::um::winevt::EvtClose;
use winapi::um::winevt::EVT_HANDLE;
use winapi::um::winevt::EvtOpenChannelEnum;
use winapi::um::winevt::EvtNextChannelPath;
use winapi::um::winevt::EvtOpenChannelConfig;
use winapi::um::winevt::EvtGetChannelConfigProperty;
use winapi::um::errhandlingapi::GetLastError;
use winapi::shared::minwindef::{DWORD};
use winapi::shared::winerror::ERROR_INSUFFICIENT_BUFFER;
use crate::errors::WinThingError;
use crate::winevt::variant::EvtVariant;
use crate::winevt::variant::VariantValue;

const CHANNEL_PROPERTIES: [(&str, u32); 21] = [
    ("EvtChannelConfigEnabled", EvtChannelConfigEnabled),
    ("EvtChannelConfigIsolation", EvtChannelConfigIsolation),
    ("EvtChannelConfigType", EvtChannelConfigType),
    ("EvtChannelConfigOwningPublisher", EvtChannelConfigOwningPublisher),
    ("EvtChannelConfigClassicEventlog", EvtChannelConfigClassicEventlog),
    ("EvtChannelConfigAccess", EvtChannelConfigAccess),
    ("EvtChannelLoggingConfigRetention", EvtChannelLoggingConfigRetention),
    ("EvtChannelLoggingConfigAutoBackup", EvtChannelLoggingConfigAutoBackup),
    ("EvtChannelLoggingConfigMaxSize", EvtChannelLoggingConfigMaxSize),
    ("EvtChannelLoggingConfigLogFilePath", EvtChannelLoggingConfigLogFilePath),
    ("EvtChannelPublishingConfigLevel", EvtChannelPublishingConfigLevel),
    ("EvtChannelPublishingConfigKeywords", EvtChannelPublishingConfigKeywords),
    ("EvtChannelPublishingConfigControlGuid", EvtChannelPublishingConfigControlGuid),
    ("EvtChannelPublishingConfigBufferSize", EvtChannelPublishingConfigBufferSize),
    ("EvtChannelPublishingConfigMinBuffers", EvtChannelPublishingConfigMinBuffers),
    ("EvtChannelPublishingConfigMaxBuffers", EvtChannelPublishingConfigMaxBuffers),
    ("EvtChannelPublishingConfigLatency", EvtChannelPublishingConfigLatency),
    ("EvtChannelPublishingConfigClockType", EvtChannelPublishingConfigClockType),
    ("EvtChannelPublishingConfigSidType", EvtChannelPublishingConfigSidType),
    ("EvtChannelPublisherList", EvtChannelPublisherList),
    ("EvtChannelPublishingConfigFileMax", EvtChannelPublishingConfigFileMax)
];


pub struct EvtHandle(EVT_HANDLE);
impl Drop for EvtHandle {
    fn drop(&mut self) {
        unsafe {
            EvtClose(
                self.0
            );
        }
    }
}


pub struct ChannelConfig {
    name: String,
    handle: EvtHandle
}
impl ChannelConfig {
    pub fn new(channel: String) -> Result<Self, WinThingError> {
        let handle = evt_open_channel_config(
            &channel
        )?;

        Ok(
            Self {
                name: channel,
                handle: handle
            }
        )
    }

    pub fn is_classic_event_log(&self) -> bool {
        match evt_get_channel_config_property(
            &self.handle, EvtChannelConfigClassicEventlog
        ) {
            Some(v) => {
                match v.get_variant_value() {
                    Ok(variant_value) => {
                        match variant_value {
                            VariantValue::Boolean(b) => b,
                            other => {
                                error!("Not expecting {:?}", other);
                                false
                            }
                        }
                    },
                    Err(e) => {
                        error!("Error getting variant value for EvtChannelConfigClassicEventlog: {:?}", e);
                        false
                    }
                }
            },
            None => false
        }
    }

    pub fn is_enabled(&self) -> bool {
        match evt_get_channel_config_property(
            &self.handle, EvtChannelConfigEnabled
        ) {
            Some(v) => {
                match v.get_variant_value() {
                    Ok(variant_value) => {
                        match variant_value {
                            VariantValue::Boolean(b) => b,
                            other => {
                                error!("Not expecting {:?}", other);
                                false
                            }
                        }
                    },
                    Err(e) => {
                        error!("Error getting variant value for EvtChannelConfigEnabled: {:?}", e);
                        false
                    }
                }
            },
            None => false
        }
    }

    pub fn to_json_value(&self) -> Result<Value, WinThingError> {
        let mut mapping = json!({});

        for (key, id) in &CHANNEL_PROPERTIES {
            let variant = match evt_get_channel_config_property(
                &self.handle, *id
            ) {
                Some(v) => v,
                None => continue
            };

            match variant.get_json_value() {
                Ok(v) => {
                    mapping[key] = v;
                },
                Err(e) => {
                    error!("Error getting variant value: {:?}", e);
                }
            }
        }

        Ok(mapping)
    }
}


pub fn get_channel_name_list() -> Vec<String> {
    let mut channel_name_list: Vec<String> = Vec::new();

    let channel_enum_handle = unsafe {
        EvtHandle(
            EvtOpenChannelEnum(
                null_mut(),
                0
            )
        )
    };

    loop {
        match evt_next_channel_id(channel_enum_handle.0) {
            None => break,
            Some(ps) => channel_name_list.push(ps)
        }
    }

    channel_name_list
}


/// wrapper for EvtGetChannelConfigProperty
fn evt_get_channel_config_property(evt_handle: &EvtHandle, property_id: EVT_CHANNEL_CONFIG_PROPERTY_ID) -> Option<EvtVariant> {
    let mut buffer_used: DWORD = 0;

    let result = unsafe {
        EvtGetChannelConfigProperty(
            evt_handle.0,
            property_id,
            0,
            0,
            null_mut(),
            &mut buffer_used
        )
    };

    // We expect this to fail but return the buffer size needed.
    if result == 0 {
        let last_error: DWORD = unsafe {
            GetLastError()
        };

        if last_error == ERROR_INSUFFICIENT_BUFFER {
            let mut buffer: Vec<u8> = vec![0; buffer_used as usize];

            let result = unsafe {
                EvtGetChannelConfigProperty(
                    evt_handle.0,
                    property_id,
                    0,
                    buffer_used,
                    buffer.as_mut_ptr() as *mut EVT_VARIANT,
                    &mut buffer_used
                )
            };

            let variant : EVT_VARIANT = unsafe {
                std::ptr::read(
                    buffer.as_ptr() as *const _
                ) 
            };

            return Some(
                EvtVariant(variant)
            );
        }
    }

    None
}

/// wrapper for EvtOpenChannelConfig
fn evt_open_channel_config(channel_path: &String) -> Result<EvtHandle, WinThingError> {
    // Create the wide string buffer
    let mut channel_path_u16 : Vec<u16> = channel_path.encode_utf16().collect();

    // Append a null wchar
    channel_path_u16.resize(channel_path.len() + 1, 0);

    let result = unsafe {
        EvtOpenChannelConfig(
            null_mut(), 
            channel_path_u16.as_ptr(), 
            0
        )
    };
    if result.is_null() {
        let last_error = unsafe {
            GetLastError()
        };

        let message = format!(
            "EvtOpenChannelConfig('{}') failed with code {}", 
            channel_path, 
            last_error
        );

        Err(
            WinThingError::winapi_error(message)
        )
    } else {
        Ok(
            EvtHandle(result)
        )
    }
}

/// wrapper for EvtNextChannelPath
fn evt_next_channel_id(channel_enum_handle: EVT_HANDLE) -> Option<String> {
    let mut buffer_used: DWORD = 0;

    let result = unsafe {
        EvtNextChannelPath(
            channel_enum_handle,
            0,
            null_mut(),
            &mut buffer_used
        )
    };

    // We expect this to fail but return the buffer size needed.
    if result == 0 {
        let last_error: DWORD = unsafe {
            GetLastError()
        };

        if last_error == ERROR_INSUFFICIENT_BUFFER {
            let mut buffer: Vec<u16> = vec![0; buffer_used as usize];

            match unsafe {
                EvtNextChannelPath(
                    channel_enum_handle,
                    buffer.len() as _,
                    buffer.as_mut_ptr() as _,
                    &mut buffer_used
                )
            } {
                0 => {
                    // This function should error here because we expected this
                    // to work. For now, we do nothing...
                },
                _ => {
                    let channel_string = OsString::from_wide(
                        &buffer[..(buffer.len()-1)]
                    ).to_string_lossy().to_string();

                    return Some(channel_string);
                }
            }
        }
    }

    None
}