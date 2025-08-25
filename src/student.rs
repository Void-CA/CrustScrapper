use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Student {
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub carnet: Option<String>,
    pub status: Option<String>,
    pub entry_date: Option<String>,
    pub shift: Option<String>,
    pub career: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct StudentRecord<'a> {
    pub code: &'a str,
    pub full_name: Option<&'a str>,
    pub email: Option<&'a str>,
    pub carnet: Option<&'a str>,
    pub status: Option<&'a str>,
    pub entry_date: Option<&'a str>,
    pub shift: Option<&'a str>,
    pub career: Option<&'a str>,
}

