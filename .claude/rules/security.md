---
paths:
  - "**/*"
---

# Güvenlik Kuralları

## Asla Yapma
- Hassas bilgileri (API key, password, token) koda yazma
- `.env` dosyalarını commit etme
- `eval()` veya `innerHTML` kullanma
- SQL sorgularını string concatenation ile oluşturma

## Her Zaman Yap
- Input validation (hem client hem server)
- Output encoding (XSS koruması)
- Parameterized queries (SQL injection koruması)
- HTTPS kullan
- CORS düzgün yapılandır

## Environment Variables
```bash
# .env.example (commit edilir)
DATABASE_URL=
API_KEY=
SECRET_KEY=

# .env.local (asla commit edilmez)
DATABASE_URL=postgresql://...
API_KEY=sk-...
```

## Hassas Dosyalar (.gitignore)
```
.env
.env.local
.env.*.local
*.pem
*.key
credentials.json
```
