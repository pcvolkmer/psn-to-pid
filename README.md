# Dienst zum internen Auflösen eines gPAS-Pseudonyms

Diese Anwendung kann ein gPAS-Pseudonym zum Originalwert auflösen.
Sie ist daher nur für interne Zwecke einer Treuhandstelle geeignet!

## Konfiguration

Beim Start der Anwendung können Parameter angegeben werden.

```
Usage: psn-to-pid [OPTIONS] --gpas-domain-name <GPAS_DOMAIN_NAME>

Options:
      --listen <LISTEN>
          Address and port for HTTP requests [env: LISTEN=] [default: [::]:3000]
      --allow-domain-override
          If set, allows overriding the domain in the HTTP request with any domain [env: ALLOW_DOMAIN_OVERRIDE=]
      --allowed-domains <ALLOWED_DOMAINS>
          If set, allows overriding the domain in the HTTP request limited to default domain and given domains [env: ALLOWED_DOMAINS=]
      --accept-invalid-certs
          If set, allows accepting invalid certificates [env: ACCEPT_INVALID_CERTS=]
      --gpas-soap-url <GPAS_SOAP_URL>
          gPAS SOAP URL [env: GPAS_SOAP_URL=] [default: https://demo.ths-greifswald.de/gpas/gpasService]
      --gpas-domain-name <GPAS_DOMAIN_NAME>
          gPAS default domain name [env: GPAS_DOMAIN_NAME=]
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

Anfrage mit *curl* unter Verwendung der konfigurierten Domain als Standard:

```bash
curl http://localhost:3000/?psn=psn_12345
```

Antwort:

```
HTTP/1.1 200 OK
content-length: 13
date: Fri, 11 Sep 2026 18:00:00 GMT

original_54321
```

Antwort, wenn das Pseudonym unbekannt ist:

```
HTTP/1.1 400 Bad Request
content-length: 38
date: Fri, 11 Sep 2026 18:00:00 GMT

invalid check digits for 'psn_unknown'
```

Wird bei einer Anfrage eine nicht existierende Domain mit angegeben, wird ein Fehler zurückgegeben.
*Achtung*: nur in Verbindung mit `--allow-domain-override` bzw. `ALLOW_DOMAIN_OVERRIDE=true` und 
`--allowed-domains` bzw `ALLOWED_DOMAINS=<domain1>;<domain2>,...` möglich.

```bash
curl http://localhost:3000/?psn=psn_12345&domain=unbekannte.domain
```

```
HTTP/1.1 400 Bad Request
content-length: 48
date: Fri, 11 Sep 2026 18:00:00 GMT

db object for domain unbekannte.domain not found
```

Wurde die Anfrage auf bestimmte Domänen eingeschränkt, wird ebenfalls ein Fehler zurückgegeben,
wenn eine nicht erlaubte Domain abgefragt wird.

```bash
curl http://localhost:3000/?psn=psn_12345&domain=forbidden.domain
```

```
HTTP/1.1 403 Forbidden
content-length: 9
date: Fri, 11 Sep 2026 18:00:00 GMT

Forbidden
```