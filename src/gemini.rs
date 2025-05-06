use self::authorisation::Authorisation;
use googleapis::google::ai::generativelanguage::v1beta::generative_service_client::GenerativeServiceClient;
use googleapis::google::ai::generativelanguage::v1beta::{
    self, GenerateContentRequest, GenerateContentResponse,
};
use tonic::service::interceptor::InterceptedService;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};

mod authorisation;
pub mod googleapis;

const ENDPOINT: &str = "https://generativelanguage.googleapis.com";

pub struct Gemini {
    client: v1beta::generative_service_client::GenerativeServiceClient<
        InterceptedService<Channel, Authorisation>,
    >,
}

impl Gemini {
    pub async fn connect(api_key: String) -> anyhow::Result<Self> {
        let tls_config = ClientTlsConfig::new().with_enabled_roots();

        let channel = Endpoint::from_static(ENDPOINT)
            .tls_config(tls_config)?
            .connect()
            .await?;

        let client = GenerativeServiceClient::with_interceptor(channel, api_key.parse()?);

        Ok(Self { client })
    }

    pub async fn generate_content(
        &mut self,
        request: GenerateContentRequest,
    ) -> anyhow::Result<GenerateContentResponse> {
        let response = self.client.generate_content(request).await?.into_inner();

        Ok(response)
    }
}
