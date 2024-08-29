use std::str::FromStr;

pub type Width = u32;
pub type Height = u32;

#[derive(Debug, Default)]
pub struct TransformationParams {
    pub width: Option<Width>,
    pub height: Option<Height>,
}

impl FromStr for TransformationParams {
    type Err = &'static str;

    fn from_str(params: &str) -> Result<Self, Self::Err> {
        let mut width: Option<Width> = None;
        let mut height: Option<Height> = None;

        for param in params.split(',') {
            if let Some((key, value)) = param.split_once('_') {
                match key {
                    "w" => width = value.parse().ok(),
                    "h" => height = value.parse().ok(),
                    _ => return Err("Invalid parameter"),
                }
            }
        }

        Ok(Self { width, height })
    }
}

impl std::fmt::Display for TransformationParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params = [
            self.width.map(|w| format!("w_{}", w)),
            self.height.map(|h| format!("h_{}", h)),
        ];

        let mut params_iter = params.into_iter().flatten();

        if let Some(first_param) = params_iter.next() {
            write!(f, "{}", first_param)?;
            for param in params_iter {
                write!(f, ",{}", param)?;
            }
        }

        Ok(())
    }
}
