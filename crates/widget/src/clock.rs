use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::System::SystemInformation::GetLocalTime;

#[derive(Debug, Clone, PartialEq)]
pub struct ClockWeatherInfo {
    pub time_str: String,
    pub date_str: String,
    pub temperature_c: f32,
    pub condition: String,
}

pub struct ClockWeatherWidget {
    temperature_c: f32,
    condition: String,
}

impl ClockWeatherWidget {
    pub fn new() -> Self {
        Self {
            temperature_c: 21.5,
            condition: "Clear Sky".to_string(),
        }
    }

    pub fn set_weather(&mut self, temp: f32, cond: &str) {
        self.temperature_c = temp;
        self.condition = cond.to_string();
    }

    pub fn poll(&self) -> ClockWeatherInfo {
        let st: SYSTEMTIME = unsafe { GetLocalTime() };

        let time_str = format!("{:02}:{:02}:{:02}", st.wHour, st.wMinute, st.wSecond);

        let day_name = match st.wDayOfWeek {
            0 => "Sunday",
            1 => "Monday",
            2 => "Tuesday",
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            _ => "Saturday",
        };

        let month_name = match st.wMonth {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            _ => "Dec",
        };

        let date_str = format!("{}, {} {} {}", day_name, st.wDay, month_name, st.wYear);

        ClockWeatherInfo {
            time_str,
            date_str,
            temperature_c: self.temperature_c,
            condition: self.condition.clone(),
        }
    }
}

impl Default for ClockWeatherWidget {
    fn default() -> Self {
        Self::new()
    }
}
