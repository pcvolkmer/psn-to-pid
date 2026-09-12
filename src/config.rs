use clap::Parser;

#[derive(Parser)]
#[command(author, version, about)]
#[command(arg_required_else_help(true))]
pub struct Config {
    #[arg(
        long,
        env = "LISTEN",
        default_value = "[::]:3000",
        help = "Address and port for HTTP requests"
    )]
    pub listen: String,
    #[arg(
        long,
        env = "ALLOW_DOMAIN_OVERRIDE",
        help = "If set, allows overriding the domain in the HTTP request"
    )]
    pub allow_domain_override: bool,
    #[arg(
        long,
        env = "GPAS_SOAP_URL",
        default_value = "https://demo.ths-greifswald.de/gpas/gpasService",
        help = "gPAS SOAP URL"
    )]
    pub gpas_soap_url: String,
    #[arg(long, env = "GPAS_DOMAIN_NAME", help = "gPAS domain name")]
    pub gpas_domain_name: String,
    #[arg(long, env = "GPAS_USERNAME", help = "gPAS HTTP-BASIC username")]
    pub gpas_username: Option<String>,
    #[arg(long, env = "GPAS_PASSWORD", help = "gPAS HTTP-BASIC password")]
    pub gpas_password: Option<String>,
}
