
#[macro_export]
macro_rules! ingress_spec {
    ($self:ident) => {{
        $self
            .ingress_list()
            .await?
            .iter()
            .map(|ingress| ingress.spec.to_owned())
            .filter(|ingress| ingress.is_some())
            .map(|ingress| ingress.unwrap())
    }}
}