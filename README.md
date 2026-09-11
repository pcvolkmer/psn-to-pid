# Dienst zum internen Auflösen eines gPAS-Pseudonyms

Diese Anwendung kann ein gPAS-Pseudonym zum Originalwert auflösen.
Sie ist daher nur für interne Zwecke einer Treuhandstelle geeignet!

## Konfiguration

Beim Start der Anwendung können Parameter angegeben werden.

```
Usage: mv64e-psn-to-pid [OPTIONS] --gpas-domain-name <GPAS_DOMAIN_NAME>

Options:
      --listen <LISTEN>
          Address and port for HTTP requests [env: LISTEN=] [default: [::]:3000]
      --gpas-soap-url <GPAS_SOAP_URL>
          gPAS SOAP URL [env: GPAS_SOAP_URL=] [default: https://demo.ths-greifswald.de/gpas/gpasService]
      --gpas-domain-name <GPAS_DOMAIN_NAME>
          gPAS domain name [env: GPAS_DOMAIN_NAME=]
      --gpas-username <GPAS_USERNAME>
          gPAS HTTP-BASIC username [env: GPAS_USERNAME=]
      --gpas-password <GPAS_PASSWORD>
          gPAS HTTP-BASIC password [env: GPAS_PASSWORD=]
  -h, --help
          Print help
  -V, --version
          Print version
```

### Beispiel für einen HTTP-Request

Anfrage mit *curl*, hier mit beiliegendem Test-File:

```bash
curl http://localhost:3000/?psn=psn_12345
```

Antwort:

```
HTTP/1.1 200 OK
content-length: ...
date: Fri, 11 Sep 2026 18:00:00 GMT

original_54321
```