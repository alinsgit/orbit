# Dev.Claude - Geliştirme Ortamı

## Proje Yapısı
```
/clients       → Müşteri projeleri
/internal      → İç projeler ve araçlar
/experiments   → Deneysel çalışmalar
/_templates    → Proje şablonları
/_shared       → Paylaşılan kaynaklar
/_docs         → Dokümantasyon
```

## Kod Standartları
- **Indentation**: 2 space
- **Naming**: kebab-case (dosyalar), camelCase (değişkenler), PascalCase (componentler)
- **Dil**: TypeScript tercih edilir
- **Formatting**: Prettier (otomatik)
- **Linting**: ESLint

## Sık Kullanılan Komutlar
- `npm run dev` - Development server
- `npm test` - Test suite
- `npm run build` - Production build
- `npm run lint` - Linting

## Git Workflow
- `main` - Production branch
- `dev` - Development branch
- Feature branches: `feature/feature-name`
- Bugfix branches: `fix/bug-description`

## Proje Oluşturma
Yeni proje için `/_templates` klasöründeki şablonları kullan.

## Notlar
- Her projede `.claude/CLAUDE.md` ile proje-spesifik bilgiler tutulur
- Hassas bilgiler `.env` dosyalarında, asla commit edilmez
- Tüm projeler git ile versiyon kontrolünde olmalı
