use atelier_common::error::{AtelierError, Result};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MediaTrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub is_playing: bool,
    pub artwork: Option<Vec<u8>>,
    pub can_play: bool,
    pub can_pause: bool,
    pub can_next: bool,
    pub can_previous: bool,
    pub can_seek: bool,
    pub position: Option<f64>,
    pub duration: Option<f64>,
}

pub struct SystemMediaMonitor;

impl SystemMediaMonitor {
    pub fn control(action: &str) -> Result<bool> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .and_then(|op| op.get())
            .map_err(|e| AtelierError::Platform(format!("Media session unavailable: {e}")))?;
        let session = manager
            .GetCurrentSession()
            .map_err(|e| AtelierError::Platform(format!("No active media session: {e}")))?;
        let operation = match action {
            "play" => session.TryPlayAsync(),
            "pause" => session.TryPauseAsync(),
            "previous" => session.TrySkipPreviousAsync(),
            "next" => session.TrySkipNextAsync(),
            _ => return Err(AtelierError::Platform("Unknown media action".into())),
        }
        .map_err(|e| AtelierError::Platform(format!("Media control unavailable: {e}")))?;
        operation
            .get()
            .map_err(|e| AtelierError::Platform(format!("Media control failed: {e}")))
    }
    pub fn seek(position: i64) -> Result<bool> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .and_then(|op| op.get())
            .map_err(|e| AtelierError::Platform(format!("Media session unavailable: {e}")))?;
        let session = manager
            .GetCurrentSession()
            .map_err(|e| AtelierError::Platform(format!("No active media session: {e}")))?;
        session
            .TryChangePlaybackPositionAsync(position)
            .and_then(|op| op.get())
            .map_err(|e| AtelierError::Platform(format!("Seeking unavailable: {e}")))
    }
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

        let controls = session
            .GetPlaybackInfo()
            .ok()
            .and_then(|i| i.Controls().ok());
        let can_play = controls
            .as_ref()
            .and_then(|c| c.IsPlayEnabled().ok())
            .unwrap_or(false);
        let can_pause = controls
            .as_ref()
            .and_then(|c| c.IsPauseEnabled().ok())
            .unwrap_or(false);
        let can_next = controls
            .as_ref()
            .and_then(|c| c.IsNextEnabled().ok())
            .unwrap_or(false);
        let can_previous = controls
            .as_ref()
            .and_then(|c| c.IsPreviousEnabled().ok())
            .unwrap_or(false);
        let can_seek = controls
            .as_ref()
            .and_then(|c| c.IsPlaybackPositionEnabled().ok())
            .unwrap_or(false);
        let timeline = session.GetTimelineProperties().ok();
        let position = timeline
            .as_ref()
            .and_then(|t| t.Position().ok())
            .map(|t| t.Duration as f64 / 10_000_000.0);
        let duration = timeline
            .as_ref()
            .and_then(|t| t.EndTime().ok())
            .map(|t| t.Duration as f64 / 10_000_000.0)
            .filter(|n| *n > 0.0);
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

        let artwork = (|| -> Option<Vec<u8>> {
            let stream = media_props
                .Thumbnail()
                .ok()?
                .OpenReadAsync()
                .ok()?
                .get()
                .ok()?;
            let size = stream.Size().ok()?;
            if size == 0 || size > 2_000_000 {
                return None;
            }
            let reader = windows::Storage::Streams::DataReader::CreateDataReader(
                &stream.GetInputStreamAt(0).ok()?,
            )
            .ok()?;
            let count = reader.LoadAsync(size as u32).ok()?.get().ok()?;
            if count != size as u32 {
                return None;
            }
            let mut bytes = vec![0; count as usize];
            reader.ReadBytes(&mut bytes).ok()?;
            if !(bytes.starts_with(&[0xff, 0xd8, 0xff])
                || bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                || bytes.starts_with(b"GIF8")
                || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP")))
            {
                return None;
            }
            Some(bytes)
        })();
        Ok(MediaTrackInfo {
            artwork,
            can_play,
            can_pause,
            can_next,
            can_previous,
            can_seek,
            position,
            duration,
            title,
            artist,
            album,
            is_playing,
        })
    }
}
