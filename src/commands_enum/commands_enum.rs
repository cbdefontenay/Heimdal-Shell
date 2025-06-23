use crate::chat::ChatConfig;

pub enum ShellCommand {
    Exit,
    Clear,
    Os,
    Tree,
    Tip,
    Search(String),
    Cd(String),
    Whoami,
    HeimdalHelp,
    Chat(ChatConfig),
    Unknown(String, Vec<String>),
}