import 'fakes.dart';

/// A small, realistic wiki: a Persian journal day with mixed-direction content, a person page,
/// a health profile with claims, and a code-bearing work page.
FakeLibrary wikiLibrary() {
  final lib = FakeLibrary();
  lib.addPage(
    'vaults/life/people/sara.md',
    'Sara',
    'سارا',
    '# Sara\n\nMy cousin. Lives in Tehran.\n\n## Timeline\n- 2026-09-23: called about the Isfahan trip',
    kind: 'person',
  );
  lib.addPage(
    'vaults/health/profile.md',
    'Health profile',
    'پروفایل سلامت',
    '# Health profile\n\n## Medications\n'
        '- Takes vitamin D 50,000 IU weekly (status:: confirmed) (src:: [[raw/2026/03/03/x|voice · 3 Mar]]) ^c-1\n'
        '- سردرد بعد از خواب کم (status:: proposed) (confidence:: low) (src:: [[raw/2026/09/23/y|صوتی · ۱ مهر]]) ^c-2\n'
        '- Takes vitamin D daily (status:: superseded) ^c-3',
    kind: 'profile',
  );
  lib.addPage(
    'vaults/life/journal/2026/2026-09-23.md',
    'Journal · 23 Sep',
    'روزنوشت ۱ مهر',
    '# ۱ مهر ۱۴۰۵\n\n'
        '## یادداشت‌ها\n'
        'امروز با [[vaults/life/people/sara|سارا]] درباره‌ی سفر اصفهان حرف زدیم؛ قرار شد Thursday راه بیفتیم.\n\n'
        'Meeting با تیم درباره‌ی migration به Postgres خوب بود.\n\n'
        '- خرید بلیت قطار\n- Pack the camera\n- [ ] تمدید گذرنامه\n- [x] Call the clinic\n\n'
        '| موضوع | Status |\n|---|---|\n| سفر | booked |\n| Migration | در حال انجام |\n\n'
        '```sql\nSELECT count(*) FROM notes; -- همه‌ی یادداشت‌ها\n```\n\n'
        '> [!conflict] From laptop\n> نسخه‌ی دیگر این پاراگراف.\n\n'
        'See [[vaults/health/profile|Health profile]] and [[nowhere]].',
    kind: 'journal-day',
    backlinks: const ['vaults/life/people/sara.md'],
  );
  return lib;
}
