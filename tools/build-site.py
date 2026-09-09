"""Gera o site (docs/) a partir de tools/site-template.html e docs/i18n.json:
docs/index.html (PT), docs/en/, docs/fr/, docs/es/ (páginas estáticas por idioma, com hreflang),
docs/sitemap.xml e docs/robots.txt. Correr depois de editar o template ou o i18n.json:

    python tools\\build-site.py
"""
import html
import json
import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SITE = 'https://arturjose0.github.io/sudomake-partition/'
LANGS = ['pt', 'en', 'fr', 'es']
LANG_NAMES = {'pt': 'Português (Angola)', 'en': 'English', 'fr': 'Français', 'es': 'Español'}
HTML_LANG = {'pt': 'pt', 'en': 'en', 'fr': 'fr', 'es': 'es'}
LOCALE = {'pt': 'pt_AO', 'en': 'en_US', 'fr': 'fr_FR', 'es': 'es_ES'}

T = json.load(open(os.path.join(ROOT, 'docs', 'i18n.json'), encoding='utf-8'))
template = open(os.path.join(ROOT, 'tools', 'site-template.html'), encoding='utf-8').read()

HTML_KEYS = {'a_p1', 'a_p2'}  # chaves cujo valor contém HTML


def translate(page, lang):
    d = T[lang]

    def rep(m):
        open_tag, key, body, close = m.group(1), m.group(2), m.group(3), m.group(4)
        if key not in d:
            return m.group(0)
        val = d[key] if key in HTML_KEYS else html.escape(d[key], quote=False)
        return open_tag + val + close

    # <tag ... data-i18n="key" ...>conteúdo</tag> (sem tags iguais aninhadas)
    return re.sub(r'(<(\w+)[^>]*\bdata-i18n="([a-z_0-9]+)"[^>]*>)(.*?)(</\2>)', lambda m: rep2(m, d), page, flags=re.S)


def rep2(m, d):
    open_tag, tag, key, body, close = m.group(1), m.group(2), m.group(3), m.group(4), m.group(5)
    if key not in d:
        return m.group(0)
    val = d[key] if key in HTML_KEYS else html.escape(d[key], quote=False)
    return open_tag + val + close


def url_for(lang):
    return SITE if lang == 'pt' else SITE + lang + '/'


def build(lang):
    d = T[lang]
    base = '' if lang == 'pt' else '../'
    page = template
    page = translate(page, lang)
    hreflang = ''.join('<link rel="alternate" hreflang="%s" href="%s">\n' % (('pt-AO' if l == 'pt' else l), url_for(l)) for l in LANGS)
    hreflang += '<link rel="alternate" hreflang="x-default" href="%s">' % SITE
    lang_links = ''.join(
        '<a href="%s" data-lang="%s" title="%s"%s>%s</a>' % (
            ((base or './') if l == 'pt' else base + l + '/'), l, LANG_NAMES[l], ' class="on"' if l == lang else '', l.upper())
        for l in LANGS)
    faq = {
        '@context': 'https://schema.org',
        '@type': 'FAQPage',
        'mainEntity': [
            {'@type': 'Question', 'name': d['q%d' % i], 'acceptedAnswer': {'@type': 'Answer', 'text': d['a%d' % i]}}
            for i in range(1, 7)
        ],
    }
    subst = {
        '{{LANG}}': HTML_LANG[lang],
        '{{LOCALE}}': LOCALE[lang],
        '{{TITLE}}': html.escape(d['meta_title'], quote=True),
        '{{DESC}}': html.escape(d['meta_desc'], quote=True),
        '{{BASE}}': base,
        '{{CANON}}': url_for(lang),
        '{{HREFLANG}}': hreflang,
        '{{LANG_LINKS}}': lang_links,
        '{{JSONLD_FAQ}}': json.dumps(faq, ensure_ascii=False, indent=1),
        '{{JSONLD_DESC}}': json.dumps(d['meta_desc'], ensure_ascii=False),
    }
    for k, v in subst.items():
        page = page.replace(k, v)
    out_dir = os.path.join(ROOT, 'docs') if lang == 'pt' else os.path.join(ROOT, 'docs', lang)
    os.makedirs(out_dir, exist_ok=True)
    with open(os.path.join(out_dir, 'index.html'), 'w', encoding='utf-8', newline='\n') as f:
        f.write(page)
    left = re.findall(r'\{\{[A-Z_]+\}\}', page)
    if left:
        print('AVISO: marcadores por substituir em', lang, set(left))
    print('gerado', lang, '->', os.path.relpath(os.path.join(out_dir, 'index.html'), ROOT))


def sitemap():
    today = __import__('datetime').date.today().isoformat()
    lines = ['<?xml version="1.0" encoding="UTF-8"?>',
             '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">']
    for l in LANGS:
        lines.append('  <url>')
        lines.append('    <loc>%s</loc>' % url_for(l))
        lines.append('    <lastmod>%s</lastmod>' % today)
        lines.append('    <changefreq>weekly</changefreq>')
        lines.append('    <priority>%s</priority>' % ('1.0' if l == 'pt' else '0.9'))
        for a in LANGS:
            lines.append('    <xhtml:link rel="alternate" hreflang="%s" href="%s"/>' % (('pt-AO' if a == 'pt' else a), url_for(a)))
        lines.append('    <xhtml:link rel="alternate" hreflang="x-default" href="%s"/>' % SITE)
        lines.append('  </url>')
    lines.append('</urlset>')
    with open(os.path.join(ROOT, 'docs', 'sitemap.xml'), 'w', encoding='utf-8', newline='\n') as f:
        f.write('\n'.join(lines) + '\n')
    with open(os.path.join(ROOT, 'docs', 'robots.txt'), 'w', encoding='utf-8', newline='\n') as f:
        f.write('User-agent: *\nAllow: /\nSitemap: %ssitemap.xml\n' % SITE)
    print('gerado sitemap.xml e robots.txt')


if __name__ == '__main__':
    for l in LANGS:
        build(l)
    sitemap()
