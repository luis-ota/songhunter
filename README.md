# SongHunter 🎵

Identificador musical via links. Cole qualquer URL (TikTok, YouTube, Instagram,
Twitter/X, Reddit, Twitch, SoundCloud etc.) e descubra quais músicas aparecem no
vídeo.

> Projeto pessoal, gratuito e mantido por anúncios.

---

## Como funciona

```
[URL do usuário]
       ↓
  yt-dlp (baixa áudio do link)
       ↓
  FFmpeg (converte para WAV 16-bit mono 44.1kHz)
       ↓
  3 identificadores em paralelo
  ├── Shazam (web API)
  ├── AcoustID (Chromaprint + MusicBrainz)
  └── ACRCloud (REST oficial)
       ↓
  Agregador ranqueia e remove duplicatas
       ↓
  Top 4 músicas possíveis
```

Toda a identificação roda em **Rust** com `tokio`/`axum`, o que torna o servidor
leve, rápido e com alto throughput.

---

## Stack

- **Backend:** Rust + Axum + Tokio + SQLx (SQLite)
- **Download:** yt-dlp (suporta 1400+ sites)
- **Áudio:** FFmpeg + Chromaprint (fpcalc)
- **Identificação:** Shazam, AcoustID, ACRCloud
- **Cache:** SQLite WAL (por hash do áudio)
- **Frontend:** HTML/CSS/JS vanilla
- **Deploy:** Docker + Docker Compose

---

## Repositório

<https://github.com/luis-ota/songhunter>

---

## Rodar localmente

### Com Docker Compose (recomendado)

1. Copie `.env.example` para `.env` e preencha suas chaves de API.
2. Execute:

```bash
docker compose up --build
```

3. Acesse `http://localhost:3000`.

### Sem Docker

Requisitos: Rust, yt-dlp, FFmpeg, fpcalc.

```bash
cp .env.example .env
# edite .env com suas chaves
cargo run --release
```

---

## Variáveis de ambiente

Veja `.env.example` para a lista completa.

| Variável | Descrição |
|---|---|
| `SONGFINDER_SHAZAM__BASE_URL` | Endpoint web do Shazam |
| `SONGFINDER_ACOUSTID__API_KEY` | Chave gratuita do AcoustID |
| `SONGFINDER_ACRCLOUD__ACCESS_KEY` | Chave do ACRCloud |
| `SONGFINDER_ACRCLOUD__ACCESS_SECRET` | Secret do ACRCloud |
| `SONGFINDER_ACRCLOUD__HOST` | Host de identificação do ACRCloud |

> Se nenhuma chave for configurada, o sistema usa o Shazam como fallback.

---

## API

### `POST /api/identify`

Inicia a identificação.

```json
{ "url": "https://www.tiktok.com/@user/video/123" }
```

Resposta:

```json
{ "task_id": "uuid", "status": "queued" }
```

### `GET /api/result/{task_id}`

Consulta o progresso/resultado.

### `GET /api/health`

Status dos provedores configurados.

---

## Modelo de negócio: gratuito com anúncios

A ferramenta é **100% gratuita** para o usuário final e é mantida por
anúncios exibidos nos espaços reservados no frontend.

### Provedores de anúncios previstos

A ideia futura é implementar um **seletor inteligente de provedores** que
escolhe o melhor anúncio para cada exibição com base em:

- CPM (custo por mil impressões)
- Fill rate (taxa de preenchimento)
- Tempo de carregamento
- Relevância para o usuário
- Pagamento mínimo/reputação

Provedores candidatos:

- Google AdSense
- Carbon Ads
- BuySellAds
- Adsterra
- PropellerAds
- Ezoic
- Outros networks locais

O seletor consulta múltiplos provedores e exibe o anúncio que maximiza a
receita esperada por impressão, garantindo que a experiência do usuário
continue rápida.

---

## Licença

MIT
