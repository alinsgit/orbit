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
- `bun dev` - Development server
- `bun test` - Test suite (Bun built-in)
- `bun run build` - Production build
- `bun run lint` - Linting
- `bun install` - Dependency installation

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
