// API client — uses mock data until backend is connected

export interface Entity {
  id: string;
  name: string;
  type: 'politician' | 'lobbyist' | 'corporation' | 'nonprofit' | 'donor';
  role?: string;
  party?: string;
  state?: string;
  connectionCount: number;
  sourceCount: number;
  flagged: boolean;
  lastUpdated: string;
}

export interface Connection {
  id: string;
  fromEntity: Entity;
  toEntity: Entity;
  type: string;
  description: string;
  amount?: number;
  date: string;
  sources: Source[];
}

export interface Source {
  id: string;
  title: string;
  url: string;
  publisher: string;
  publishedDate: string;
  type: 'filing' | 'news' | 'government' | 'court' | 'financial';
}

export interface TimingCorrelation {
  id: string;
  official: string;
  officialId: string;
  eventA: string;
  eventADate: string;
  eventB: string;
  eventBDate: string;
  daysBetween: number;
  correlationType: 'vote' | 'donation' | 'meeting' | 'regulation' | 'appointment';
  flagged: boolean;
  sources: Source[];
}

export interface ConductRow {
  id: string;
  officialAction: string;
  official: string;
  officialId: string;
  date: string;
  source: Source;
  equivalentPrivateConduct: string;
  consequence: string;
}

export interface SearchFilters {
  entityTypes?: Entity['type'][];
  dateFrom?: string;
  dateTo?: string;
  flaggedOnly?: boolean;
  query?: string;
}

export interface SubmissionData {
  entityName: string;
  connectionType: string;
  description: string;
  sourceUrl: string;
  referenceDetail: string;
  submitterNote?: string;
}

// ─── Mock data ────────────────────────────────────────────────────────────────

const MOCK_SOURCES: Source[] = [
  {
    id: 's1',
    title: 'FEC Filing Q3 2024 — Defense PAC Contributions',
    url: 'https://www.fec.gov/data/receipts/?committee_id=C00123456',
    publisher: 'Federal Election Commission',
    publishedDate: '2024-10-15',
    type: 'filing',
  },
  {
    id: 's2',
    title: 'Senate Vote Record SB-2024-0042: Defense Appropriations',
    url: 'https://www.congress.gov/bill/118th-congress/senate-bill/42',
    publisher: 'Congress.gov',
    publishedDate: '2024-03-22',
    type: 'government',
  },
  {
    id: 's3',
    title: 'Lobbyist Disclosure Report — Aerospace Consortium',
    url: 'https://lda.senate.gov/filings/public/filing/2024-aerospace-consortium/',
    publisher: 'U.S. Senate Lobbying Disclosure',
    publishedDate: '2024-01-20',
    type: 'filing',
  },
  {
    id: 's4',
    title: 'Campaign Finance Disclosure: Senator R. Walsh 2024',
    url: 'https://www.opensecrets.org/politicians/contrib.php?cid=N00003679',
    publisher: 'OpenSecrets',
    publishedDate: '2024-11-01',
    type: 'financial',
  },
  {
    id: 's5',
    title: 'DOJ Antitrust Investigation: PharmaCorp Merger — Case 24-cv-0091',
    url: 'https://www.courtlistener.com/docket/24-cv-0091/',
    publisher: 'CourtListener / PACER',
    publishedDate: '2024-07-08',
    type: 'court',
  },
  {
    id: 's6',
    title: "Senator Walsh's $2.4M from Defense Sector — Reuters Investigation",
    url: 'https://www.reuters.com/investigates/defense-pac-senators-2024/',
    publisher: 'Reuters',
    publishedDate: '2024-08-15',
    type: 'news',
  },
];

const MOCK_ENTITIES: Entity[] = [
  {
    id: 'e1',
    name: 'Senator Richard Walsh',
    type: 'politician',
    role: 'U.S. Senator',
    party: 'Republican',
    state: 'TX',
    connectionCount: 47,
    sourceCount: 23,
    flagged: true,
    lastUpdated: '2024-11-15',
  },
  {
    id: 'e2',
    name: 'Aerospace Defense PAC',
    type: 'corporation',
    role: 'Political Action Committee',
    connectionCount: 128,
    sourceCount: 67,
    flagged: true,
    lastUpdated: '2024-10-30',
  },
  {
    id: 'e3',
    name: 'Representative Diana Chen',
    type: 'politician',
    role: 'U.S. Representative',
    party: 'Democrat',
    state: 'CA',
    connectionCount: 31,
    sourceCount: 18,
    flagged: false,
    lastUpdated: '2024-11-10',
  },
  {
    id: 'e4',
    name: 'Marcus Leland',
    type: 'lobbyist',
    role: 'Senior Lobbyist',
    connectionCount: 89,
    sourceCount: 44,
    flagged: true,
    lastUpdated: '2024-11-01',
  },
  {
    id: 'e5',
    name: 'PharmaCorp Industries',
    type: 'corporation',
    role: 'Pharmaceutical Manufacturer',
    connectionCount: 203,
    sourceCount: 91,
    flagged: true,
    lastUpdated: '2024-11-20',
  },
  {
    id: 'e6',
    name: 'Governor Patricia Monroe',
    type: 'politician',
    role: 'State Governor',
    party: 'Democrat',
    state: 'FL',
    connectionCount: 56,
    sourceCount: 29,
    flagged: false,
    lastUpdated: '2024-10-22',
  },
];

const MOCK_TIMING: TimingCorrelation[] = [
  {
    id: 't1',
    official: 'Sen. Richard Walsh',
    officialId: 'e1',
    eventA: 'Received $450,000 from Aerospace Defense PAC',
    eventADate: '2024-02-14',
    eventB: 'Voted YES on SB-2024-0042 ($8.2B Defense Appropriations)',
    eventBDate: '2024-03-22',
    daysBetween: 37,
    correlationType: 'donation',
    flagged: true,
    sources: [MOCK_SOURCES[0], MOCK_SOURCES[1]],
  },
  {
    id: 't2',
    official: 'Rep. Diana Chen',
    officialId: 'e3',
    eventA: 'Private meeting with PharmaCorp CEO (LDA disclosure)',
    eventADate: '2024-05-10',
    eventB: 'Co-sponsored HR-2024-0189: Drug Price Deregulation Act',
    eventBDate: '2024-05-28',
    daysBetween: 18,
    correlationType: 'meeting',
    flagged: true,
    sources: [MOCK_SOURCES[2]],
  },
  {
    id: 't3',
    official: 'Gov. Patricia Monroe',
    officialId: 'e6',
    eventA: 'Marcus Leland bundled $1.2M for Monroe campaign',
    eventADate: '2023-10-01',
    eventB: 'Monroe appointed Leland\'s client as State Treasurer',
    eventBDate: '2024-01-15',
    daysBetween: 106,
    correlationType: 'appointment',
    flagged: true,
    sources: [MOCK_SOURCES[3]],
  },
  {
    id: 't4',
    official: 'Sen. Richard Walsh',
    officialId: 'e1',
    eventA: 'Attended Aerospace Consortium annual gala',
    eventADate: '2024-06-05',
    eventB: 'Blocked floor vote on defense contractor oversight amendment',
    eventBDate: '2024-06-19',
    daysBetween: 14,
    correlationType: 'vote',
    flagged: true,
    sources: [MOCK_SOURCES[5]],
  },
  {
    id: 't5',
    official: 'Rep. Diana Chen',
    officialId: 'e3',
    eventA: 'Town hall on healthcare access (public record)',
    eventADate: '2024-09-12',
    eventB: 'Voted YES on Medicare expansion amendment',
    eventBDate: '2024-09-30',
    daysBetween: 18,
    correlationType: 'vote',
    flagged: false,
    sources: [MOCK_SOURCES[1]],
  },
];

const MOCK_CONDUCT: ConductRow[] = [
  {
    id: 'c1',
    officialAction: 'Accepted $450,000 PAC donation from defense contractor, then voted to award same contractor $8.2B contract',
    official: 'Sen. Richard Walsh',
    officialId: 'e1',
    date: '2024-03-22',
    source: MOCK_SOURCES[0],
    equivalentPrivateConduct: 'Receiving payment from a party and then awarding them a contract in a professional capacity',
    consequence: 'No investigation. Reelected. Contractor received full award.',
  },
  {
    id: 'c2',
    officialAction: 'Met privately with pharmaceutical CEO, then co-sponsored bill removing drug pricing controls affecting CEO\'s company',
    official: 'Rep. Diana Chen',
    officialId: 'e3',
    date: '2024-05-28',
    source: MOCK_SOURCES[2],
    equivalentPrivateConduct: 'Undisclosed conflict of interest in regulatory decision-making',
    consequence: 'Bill passed committee. Under ethics review.',
  },
  {
    id: 'c3',
    officialAction: 'Appointed major donor\'s business associate to cabinet position without competitive process',
    official: 'Gov. Patricia Monroe',
    officialId: 'e6',
    date: '2024-01-15',
    source: MOCK_SOURCES[3],
    equivalentPrivateConduct: 'Nepotism / quid pro quo employment arrangement',
    consequence: 'Appointee confirmed. FOIA request filed. Ongoing public scrutiny.',
  },
  {
    id: 'c4',
    officialAction: 'Used official government travel budget for 14 trips to lobbying firm\'s headquarter city',
    official: 'Sen. Richard Walsh',
    officialId: 'e1',
    date: '2024-08-01',
    source: MOCK_SOURCES[5],
    equivalentPrivateConduct: 'Misappropriation of company funds for personal/external benefit',
    consequence: 'GAO inquiry requested. No disciplinary action taken.',
  },
];

// ─── API functions ─────────────────────────────────────────────────────────

export async function searchEntities(
  query: string,
  filters: SearchFilters = {}
): Promise<Entity[]> {
  await new Promise((r) => setTimeout(r, 0)); // simulate async
  let results = [...MOCK_ENTITIES];

  if (query) {
    const q = query.toLowerCase();
    results = results.filter(
      (e) =>
        e.name.toLowerCase().includes(q) ||
        e.role?.toLowerCase().includes(q) ||
        e.type.toLowerCase().includes(q)
    );
  }

  if (filters.entityTypes?.length) {
    results = results.filter((e) => filters.entityTypes!.includes(e.type));
  }

  if (filters.flaggedOnly) {
    results = results.filter((e) => e.flagged);
  }

  return results;
}

export async function getEntity(id: string): Promise<Entity | null> {
  await new Promise((r) => setTimeout(r, 0));
  return MOCK_ENTITIES.find((e) => e.id === id) ?? null;
}

export async function getEntityConnections(entityId: string): Promise<Connection[]> {
  await new Promise((r) => setTimeout(r, 0));
  const entity = MOCK_ENTITIES.find((e) => e.id === entityId);
  if (!entity) return [];
  return [
    {
      id: 'conn1',
      fromEntity: entity,
      toEntity: MOCK_ENTITIES[1],
      type: 'financial',
      description: '$450,000 PAC contribution via Aerospace Defense PAC',
      amount: 450000,
      date: '2024-02-14',
      sources: [MOCK_SOURCES[0]],
    },
    {
      id: 'conn2',
      fromEntity: entity,
      toEntity: MOCK_ENTITIES[3],
      type: 'meeting',
      description: 'Recorded meeting — Senate lobbying disclosure database',
      date: '2024-06-05',
      sources: [MOCK_SOURCES[2]],
    },
  ];
}

export async function getTimingCorrelations(
  filters: { flaggedOnly?: boolean } = {}
): Promise<TimingCorrelation[]> {
  await new Promise((r) => setTimeout(r, 0));
  if (filters.flaggedOnly) return MOCK_TIMING.filter((t) => t.flagged);
  return MOCK_TIMING;
}

export async function getConductRows(): Promise<ConductRow[]> {
  await new Promise((r) => setTimeout(r, 0));
  return MOCK_CONDUCT;
}

export async function getStats(): Promise<{
  entities: number;
  connections: number;
  sources: number;
  flagged: number;
}> {
  await new Promise((r) => setTimeout(r, 0));
  return { entities: 2847, connections: 14203, sources: 8916, flagged: 342 };
}

export async function submitConnection(data: SubmissionData): Promise<{ success: boolean; id?: string; error?: string }> {
  await new Promise((r) => setTimeout(r, 300));
  if (!data.sourceUrl || !data.referenceDetail) {
    return { success: false, error: 'Source URL and reference detail are required.' };
  }
  return { success: true, id: `sub_${Date.now()}` };
}
