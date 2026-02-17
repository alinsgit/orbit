// Nav scroll effect
const nav = document.getElementById('nav');
window.addEventListener('scroll', () => {
    nav.classList.toggle('scrolled', window.scrollY > 20);
});

// Scroll-in animations (IntersectionObserver)
const observer = new IntersectionObserver((entries) => {
    entries.forEach((entry, i) => {
        if (entry.isIntersecting) {
            // Stagger delay for siblings
            const parent = entry.target.parentElement;
            const siblings = Array.from(parent.children).filter(c =>
                c.classList.contains('feature-card') ||
                c.classList.contains('dl-card') ||
                c.classList.contains('svc-chip')
            );
            const idx = siblings.indexOf(entry.target);
            const delay = idx >= 0 ? idx * 80 : 0;

            setTimeout(() => {
                entry.target.classList.add('visible');
            }, delay);

            observer.unobserve(entry.target);
        }
    });
}, { threshold: 0.1, rootMargin: '0px 0px -50px 0px' });

document.querySelectorAll('.feature-card, .dl-card, .svc-chip, .terminal-window').forEach(el => {
    observer.observe(el);
});

// Terminal typing animation
function typeTerminal() {
    const lines = document.querySelectorAll('.term-line');
    lines.forEach(line => line.style.opacity = '0');

    lines.forEach((line, i) => {
        setTimeout(() => {
            line.style.opacity = '1';
            line.style.transition = 'opacity 0.3s ease';
        }, i * 400);
    });
}

// Start terminal animation when visible
const termObserver = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
        if (entry.isIntersecting) {
            typeTerminal();
            termObserver.unobserve(entry.target);
        }
    });
}, { threshold: 0.3 });

const termBody = document.querySelector('.term-body');
if (termBody) termObserver.observe(termBody);

// Detect platform and highlight download card
function detectPlatform() {
    const ua = navigator.userAgent.toLowerCase();
    let platform = 'windows';
    if (ua.includes('mac')) platform = 'macos';
    else if (ua.includes('linux')) platform = 'linux';

    const card = document.getElementById('dl-' + platform);
    if (card) {
        card.style.borderColor = 'var(--emerald)';
        card.style.boxShadow = '0 0 40px rgba(16,185,129,0.1)';
        const badge = card.querySelector('.dl-badge');
        if (badge) badge.textContent = '★ Recommended';
    }
}

detectPlatform();

// Smooth scroll for nav links
document.querySelectorAll('a[href^="#"]').forEach(a => {
    a.addEventListener('click', (e) => {
        const target = document.querySelector(a.getAttribute('href'));
        if (target) {
            e.preventDefault();
            target.scrollIntoView({ behavior: 'smooth' });
        }
    });
});
