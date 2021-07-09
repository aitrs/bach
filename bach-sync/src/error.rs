#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Kind {
    IncompatCommand,
    WrongSourcePath,
    WrongDestPath,
    WrongNetworkReach,
    SshRequiringPassword,
    BadReturnCode,
    BadPath,
    BadCmd,
    ThreadError,
    TimeoutError,
    RecvError,
    Generic,
}
#[derive(Copy, Clone, Debug)]
pub struct Error {
    kind: Kind,
    code: Option<u16>,
    message: Option<&'static str>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bachd Error")
    }
}

impl std::error::Error for Error {}

impl std::convert::From<std::sync::mpsc::RecvError> for Error {
    fn from(_: std::sync::mpsc::RecvError) -> Self {
        Error::new(Kind::RecvError, None, None)
    }
}

impl std::convert::From<Box<dyn std::any::Any + Send>> for Error {
    fn from(_: Box<dyn std::any::Any + Send>) -> Self {
        Error::new(Kind::BadCmd, None, Some("Impossible de joindre"))
    }
}

impl Error {
    pub fn new(k: Kind, c: Option<u16>, m: Option<&'static str>) -> Error {
        Error {
            kind: k,
            code: c,
            message: m,
        }
    }

    pub fn new_incompat(m: &'static str) -> Error {
        Error::new(Kind::IncompatCommand, None, Some(m))
    }
    pub fn new_wrsource(m: &'static str) -> Error {
        Error::new(Kind::WrongSourcePath, None, Some(m))
    }
    pub fn new_wrdest(m: &'static str) -> Error {
        Error::new(Kind::WrongDestPath, None, Some(m))
    }
    pub fn new_wrnet(m: &'static str) -> Error {
        Error::new(Kind::WrongNetworkReach, None, Some(m))
    }
    pub fn new_sshpwd(m: &'static str) -> Error {
        Error::new(Kind::SshRequiringPassword, None, Some(m))
    }
    pub fn new_badret(c: u16) -> Error {
        Error::new(Kind::BadReturnCode, Some(c), None)
    }

    pub fn to_string(&self) -> String {
        let mut kind_str = String::new();
        match self.kind {
            Kind::IncompatCommand => kind_str.push_str("Command Incompatible"),
            Kind::WrongSourcePath => kind_str.push_str("Chemin Source Erroné"),
            Kind::WrongDestPath => kind_str.push_str("Chemin de destination éronné"),
            Kind::WrongNetworkReach => kind_str.push_str("Cible réseau éronnée"),
            Kind::SshRequiringPassword => kind_str.push_str("SSH demande un mot de passe"),
            Kind::BadReturnCode => kind_str.push_str("Mauvais code retour"),
            Kind::BadPath => kind_str.push_str("Mauvais Path"),
            Kind::BadCmd => kind_str.push_str("Erreur de commande"),
            Kind::ThreadError => kind_str.push_str("Erreur de threading"),
            Kind::TimeoutError => kind_str.push_str("Delai d'attente dépassé"),
            Kind::RecvError => kind_str.push_str("Erreur de réception"),
            Kind::Generic => kind_str.push_str("Erreur"),
        }

        match self.code {
            Some(n) => kind_str.push_str(&std::format!(" :: Code: {}", n)),
            None => (),
        }

        match &self.message {
            Some(s) => kind_str.push_str(&std::format!(" :: Message: {}", s)),
            None => (),
        }

        kind_str
    }
}
