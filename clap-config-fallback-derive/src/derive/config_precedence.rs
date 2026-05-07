use darling::FromMeta;

#[derive(Copy, Clone, Default, FromMeta)]
#[darling(default)]
pub enum ConfigPrecedence {
    AfterDefault,
    #[default]
    BeforeDefault,
    BeforeEnv,
}
