# Next.js E-Commerce Template

## Kullanım
```bash
# Bu template'i yeni projeye kopyala
cp -r _templates/nextjs-ecommerce clients/yeni-proje

# Bağımlılıkları yükle
cd clients/yeni-proje
npm install

# Development server
npm run dev
```

## Önerilen Stack
- **Framework**: Next.js 14+ (App Router)
- **Styling**: Tailwind CSS
- **State**: Zustand veya React Context
- **Database**: PostgreSQL + Prisma
- **Auth**: NextAuth.js
- **Ödeme**: iyzico / Stripe

## Klasör Yapısı
```
src/
├── app/
│   ├── (shop)/           # Mağaza sayfaları
│   │   ├── page.tsx      # Ana sayfa
│   │   ├── products/     # Ürün listesi
│   │   ├── product/[slug]/ # Ürün detay
│   │   └── cart/         # Sepet
│   ├── (auth)/           # Auth sayfaları
│   └── api/              # API routes
├── components/
│   ├── product/
│   ├── cart/
│   └── checkout/
├── lib/
│   ├── db.ts             # Database client
│   └── utils.ts
└── types/
    └── index.ts
```

## Gerekli Env Variables
```
DATABASE_URL=
NEXTAUTH_SECRET=
NEXTAUTH_URL=
IYZICO_API_KEY=
IYZICO_SECRET_KEY=
```
