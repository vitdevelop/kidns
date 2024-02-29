use crate::proxy::server::proxy::Proxy;
use anyhow::anyhow;
use rcgen::{Certificate, CertificateParams, DnType, KeyPair, PKCS_RSA_SHA256, SanType};
use std::env;
use std::fs::File;
use std::io::Read;
use std::ops::Add;
use rsa::pkcs8::EncodePrivateKey;
use rsa::RsaPrivateKey;
use time::{Duration, OffsetDateTime};

/// Supported Keypair Algorithms
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyPairAlgorithm {
    #[allow(unused)]
    Ed25519,
    #[allow(unused)]
    EcdsaP256,
    #[allow(unused)]
    EcdsaP384,
    #[allow(unused)]
    RSA,
}

impl KeyPairAlgorithm {
    /// Return an `rcgen::KeyPair` for the given varient
    fn to_key_pair(self) -> anyhow::Result<KeyPair> {
        match self {
            KeyPairAlgorithm::Ed25519 => {
                use ring::signature::Ed25519KeyPair;

                let rng = ring::rand::SystemRandom::new();
                let alg = &rcgen::PKCS_ED25519;
                let pkcs8_bytes =
                    Ed25519KeyPair::generate_pkcs8(&rng).or(Err(rcgen::Error::RingUnspecified))?;

                Ok(KeyPair::from_der_and_sign_algo(pkcs8_bytes.as_ref(), alg)?)
            }
            KeyPairAlgorithm::EcdsaP256 => {
                use ring::signature::EcdsaKeyPair;
                use ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING;

                let rng = ring::rand::SystemRandom::new();
                let alg = &rcgen::PKCS_ECDSA_P256_SHA256;
                let pkcs8_bytes =
                    EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
                        .or(Err(rcgen::Error::RingUnspecified))?;
                Ok(KeyPair::from_der_and_sign_algo(pkcs8_bytes.as_ref(), alg)?)
            }
            KeyPairAlgorithm::EcdsaP384 => {
                use ring::signature::EcdsaKeyPair;
                use ring::signature::ECDSA_P384_SHA384_ASN1_SIGNING;

                let rng = ring::rand::SystemRandom::new();
                let alg = &rcgen::PKCS_ECDSA_P384_SHA384;
                let pkcs8_bytes =
                    EcdsaKeyPair::generate_pkcs8(&ECDSA_P384_SHA384_ASN1_SIGNING, &rng)
                        .or(Err(rcgen::Error::RingUnspecified))?;

                Ok(KeyPair::from_der_and_sign_algo(pkcs8_bytes.as_ref(), alg)?)
            }
            KeyPairAlgorithm::RSA => {
                let mut rng = rand::rngs::OsRng;
                let bits = 3072;
                let private_key = RsaPrivateKey::new(&mut rng, bits)?;
                let private_key_der = private_key.to_pkcs8_der()?;

                Ok(rcgen::KeyPair::try_from(private_key_der.as_bytes())?)
            }
        }
    }
}

impl Proxy {
    /// Generate CA signed client cert, return (key, cert) in pem format
    pub(crate) fn generate_signed_cert(&self, domain: &str) -> anyhow::Result<(String, String)> {
        let ca = self
            .root_cert
            .as_ref()
            .ok_or(anyhow!("Unable to generate CA certificate, is empty"))?;

        let mut client_cert_params = CertificateParams::default();
        // client_cert_params.alg = &PKCS_ECDSA_P384_SHA384;
        // client_cert_params.key_pair = Some(KeyPairAlgorithm::EcdsaP384.to_key_pair()?);
        client_cert_params.alg = &PKCS_RSA_SHA256;
        client_cert_params.key_pair = Some(KeyPairAlgorithm::RSA.to_key_pair()?);
        client_cert_params
            .distinguished_name
            .push(DnType::CommonName, domain);

        client_cert_params.subject_alt_names = vec![SanType::DnsName(domain.to_string())];

        client_cert_params.not_before = OffsetDateTime::now_utc();
        client_cert_params.not_after = OffsetDateTime::now_utc().add(Duration::days(365));

        let client_cert = Certificate::from_params(client_cert_params)?;
        let client_crt = client_cert.serialize_pem_with_signer(ca)?;
        let client_key = client_cert.serialize_private_key_pem();

        Ok((client_key, client_crt))
    }
}

pub(crate) async fn get_root_ca_params(
    key_path: &String,
    cert_path: &String,
) -> anyhow::Result<Certificate> {
    let parent = env::current_dir()?;

    let cert_path = parent.join(cert_path).to_str().unwrap().to_string();
    let key_path = parent.join(key_path).to_str().unwrap().to_string();

    let mut cert_file = String::new();
    File::open(cert_path)?.read_to_string(&mut cert_file)?;

    let mut key_file = String::new();
    File::open(key_path)?.read_to_string(&mut key_file)?;

    Ok(Certificate::from_params(
        CertificateParams::from_ca_cert_pem(
            cert_file.as_str(),
            KeyPair::from_pem(key_file.as_str())?,
        )?,
    )?)
}
