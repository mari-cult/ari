use self::authorisation::Authorisation;
use googleapis::google::ai::generativelanguage::v1alpha::{self, BidiGenerateContentClientMessage};
use googleapis::google::ai::generativelanguage::v1beta::{
    self, GenerateContentRequest, GenerateContentResponse,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_stream::wrappers::UnboundedReceiverStream;
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

        let client = v1beta::generative_service_client::GenerativeServiceClient::with_interceptor(
            channel,
            api_key.parse()?,
        );

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

pub struct GeminiLive {
    client: v1alpha::generative_service_client::GenerativeServiceClient<
        InterceptedService<Channel, Authorisation>,
    >,
}

impl GeminiLive {
    pub async fn connect(api_key: String) -> anyhow::Result<Self> {
        let tls_config = ClientTlsConfig::new().with_enabled_roots();

        let channel = Endpoint::from_static(ENDPOINT)
            .tls_config(tls_config)?
            .connect()
            .await?;

        let client = v1alpha::generative_service_client::GenerativeServiceClient::with_interceptor(
            channel,
            api_key.parse()?,
        );

        Ok(Self { client })
    }

    pub async fn bidi(
        &mut self,
        stream: UnboundedReceiver<BidiGenerateContentClientMessage>,
    ) -> anyhow::Result<tonic::Streaming<v1alpha::BidiGenerateContentServerMessage>> {
        let stream = self
            .client
            .bidi_generate_content(UnboundedReceiverStream::new(stream))
            .await?
            .into_inner();

        Ok(stream)
    }
}
