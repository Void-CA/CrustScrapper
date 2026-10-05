use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Student {
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub carnet: Option<String>,
    pub status: Option<String>,
    pub entry_date: Option<String>,
    pub shift: Option<String>,
    pub career: Option<String>,
}

impl Student {
    pub fn has_data(&self) -> bool {
        self.full_name.is_some()
            || self.email.is_some()
            || self.carnet.is_some()
            || self.status.is_some()
            || self.entry_date.is_some()
            || self.shift.is_some()
            || self.career.is_some()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StudentRow {
    pub code: String,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub carnet: Option<String>,
    pub status: Option<String>,
    pub entry_date: Option<String>,
    pub shift: Option<String>,
    pub career: Option<String>,
}

impl StudentRow {
    pub fn from_student(code: impl Into<String>, student: &Student) -> Self {
        Self {
            code: code.into(),
            full_name: student.full_name.clone(),
            email: student.email.clone(),
            carnet: student.carnet.clone(),
            status: student.status.clone(),
            entry_date: student.entry_date.clone(),
            shift: student.shift.clone(),
            career: student.career.clone(),
        }
    }

    pub fn identifiers(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.code.as_str()).chain(self.carnet.as_deref())
    }
}
