use atelier_common::error::{AtelierError, Result};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaTrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub is_playing: bool,
}

pub struct SystemMediaMonitor;

impl SystemMediaMonitor {
    pub fn poll() -> Result<MediaTrackInfo> {
        let async_op = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .map_err(|e| AtelierError::Platform(format!("GSMTC request failed: {e}")))?;

        let manager = async_op.get().map_err(|e| {
            AtelierError::Platform(format!("Failed to retrieve GSMTC manager: {e}"))
        })?;

        let session = match manager.GetCurrentSession() {
            Ok(s) => s,
            Err(_) => return Ok(MediaTrackInfo::default()),
        };

        let is_playing = match session.GetPlaybackInfo() {
            Ok(info) => match info.PlaybackStatus() {
                Ok(status) => {
                    status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                }
                Err(_) => false,
            },
            Err(_) => false,
        };

        let media_props = match session.TryGetMediaPropertiesAsync() {
            Ok(async_props) => match async_props.get() {
                Ok(props) => props,
                Err(_) => {
                    return Ok(MediaTrackInfo {
                        is_playing,
                        ..Default::default()
                    })
                }
            },
            Err(_) => {
                return Ok(MediaTrackInfo {
                    is_playing,
                    ..Default::default()
                })
            }
        };

        let title = media_props
            .Title()
            .map(|s| s.to_string())
            .unwrap_or_default();

        let artist = media_props
            .Artist()
            .map(|s| s.to_string())
            .unwrap_or_default();

        let album = media_props
            .AlbumTitle()
            .map(|s| s.to_string())
            .unwrap_or_default();

        Ok(MediaTrackInfo {
            title,
            artist,
            album,
            is_playing,
        })
    }
}
