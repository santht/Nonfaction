-- Seed data for Nonfaction D1 database
-- Phase 1: Federal top-level officials + Arizona delegation
-- Entity data follows the FollowTheMoney model

-- ════════════════════════════════════════════════════════════════════════════
-- EXECUTIVE BRANCH
-- ════════════════════════════════════════════════════════════════════════════

INSERT OR IGNORE INTO entities (id, entity_type, version, data) VALUES
-- President & VP
('00000001-0000-0000-0000-000000000001', 'Person', 1, '{"name":"Joseph R. Biden Jr.","role":"President of the United States","party":"Democratic","state":"Delaware","birth_year":1942,"in_office_since":"2021-01-20","source_url":"https://www.whitehouse.gov/administration/president-biden/"}'),
('00000001-0000-0000-0000-000000000002', 'Person', 1, '{"name":"Kamala D. Harris","role":"Vice President of the United States","party":"Democratic","state":"California","birth_year":1964,"in_office_since":"2021-01-20","source_url":"https://www.whitehouse.gov/administration/vice-president-harris/"}'),

-- Key Cabinet
('00000001-0000-0000-0000-000000000010', 'Person', 1, '{"name":"Antony J. Blinken","role":"Secretary of State","party":"Democratic","birth_year":1962,"in_office_since":"2021-01-26","source_url":"https://www.state.gov/secretary/"}'),
('00000001-0000-0000-0000-000000000011', 'Person', 1, '{"name":"Janet L. Yellen","role":"Secretary of the Treasury","party":"Democratic","birth_year":1946,"in_office_since":"2021-01-26","source_url":"https://home.treasury.gov/about/general-information/officials/janet-yellen"}'),
('00000001-0000-0000-0000-000000000012', 'Person', 1, '{"name":"Lloyd J. Austin III","role":"Secretary of Defense","party":"Democratic","birth_year":1953,"in_office_since":"2021-01-22","source_url":"https://www.defense.gov/About/Biographies/Biography/Article/2556767/"}'),
('00000001-0000-0000-0000-000000000013', 'Person', 1, '{"name":"Merrick B. Garland","role":"Attorney General","party":"Democratic","birth_year":1952,"in_office_since":"2021-03-11","source_url":"https://www.justice.gov/ag/bio"}');

-- ════════════════════════════════════════════════════════════════════════════
-- SUPREME COURT
-- ════════════════════════════════════════════════════════════════════════════

INSERT OR IGNORE INTO entities (id, entity_type, version, data) VALUES
('00000002-0000-0000-0000-000000000001', 'Person', 1, '{"name":"John G. Roberts Jr.","role":"Chief Justice of the United States","appointed_by":"George W. Bush","confirmed_year":2005,"source_url":"https://www.supremecourt.gov/about/biographies.aspx"}'),
('00000002-0000-0000-0000-000000000002', 'Person', 1, '{"name":"Clarence Thomas","role":"Associate Justice","appointed_by":"George H.W. Bush","confirmed_year":1991,"source_url":"https://www.supremecourt.gov/about/biographies.aspx"}'),
('00000002-0000-0000-0000-000000000003', 'Person', 1, '{"name":"Samuel A. Alito Jr.","role":"Associate Justice","appointed_by":"George W. Bush","confirmed_year":2006,"source_url":"https://www.supremecourt.gov/about/biographies.aspx"}'),
('00000002-0000-0000-0000-000000000004', 'Person', 1, '{"name":"Sonia Sotomayor","role":"Associate Justice","appointed_by":"Barack Obama","confirmed_year":2009,"source_url":"https://www.supremecourt.gov/about/biographies.aspx"}'),
('00000002-0000-0000-0000-000000000005', 'Person', 1, '{"name":"Elena Kagan","role":"Associate Justice","appointed_by":"Barack Obama","confirmed_year":2010,"source_url":"https://www.supremecourt.gov/about/biographies.aspx"}'),
('00000002-0000-0000-0000-000000000006', 'Person', 1, '{"name":"Neil M. Gorsuch","role":"Associate Justice","appointed_by":"Donald Trump","confirmed_year":2017,"source_url":"https://www.supremecourt.gov/about/biographies.aspx"}'),
('00000002-0000-0000-0000-000000000007', 'Person', 1, '{"name":"Brett M. Kavanaugh","role":"Associate Justice","appointed_by":"Donald Trump","confirmed_year":2018,"source_url":"https://www.supremecourt.gov/about/biographies.aspx"}'),
('00000002-0000-0000-0000-000000000008', 'Person', 1, '{"name":"Amy Coney Barrett","role":"Associate Justice","appointed_by":"Donald Trump","confirmed_year":2020,"source_url":"https://www.supremecourt.gov/about/biographies.aspx"}'),
('00000002-0000-0000-0000-000000000009', 'Person', 1, '{"name":"Ketanji Brown Jackson","role":"Associate Justice","appointed_by":"Joseph Biden","confirmed_year":2022,"source_url":"https://www.supremecourt.gov/about/biographies.aspx"}');

-- ════════════════════════════════════════════════════════════════════════════
-- ARIZONA DELEGATION (119th Congress)
-- ════════════════════════════════════════════════════════════════════════════

-- Arizona Senators
INSERT OR IGNORE INTO entities (id, entity_type, version, data) VALUES
('00000003-0001-0000-0000-000000000001', 'Person', 1, '{"name":"Kyrsten Sinema","role":"U.S. Senator","party":"Independent","state":"Arizona","in_office_since":"2019-01-03","fec_candidate_id":"S8AZ00197","source_url":"https://www.congress.gov/member/kyrsten-sinema/S001191"}'),
('00000003-0001-0000-0000-000000000002', 'Person', 1, '{"name":"Mark Kelly","role":"U.S. Senator","party":"Democratic","state":"Arizona","in_office_since":"2020-12-02","fec_candidate_id":"S0AZ00243","source_url":"https://www.congress.gov/member/mark-kelly/K000377"}');

-- Arizona Representatives (9 districts)
INSERT OR IGNORE INTO entities (id, entity_type, version, data) VALUES
('00000003-0002-0001-0000-000000000001', 'Person', 1, '{"name":"David Schweikert","role":"U.S. Representative","party":"Republican","state":"Arizona","district":1,"in_office_since":"2011-01-03","fec_candidate_id":"H0AZ05094","source_url":"https://www.congress.gov/member/david-schweikert/S001183"}'),
('00000003-0002-0002-0000-000000000001', 'Person', 1, '{"name":"Eli Crane","role":"U.S. Representative","party":"Republican","state":"Arizona","district":2,"in_office_since":"2023-01-03","fec_candidate_id":"H2AZ02119","source_url":"https://www.congress.gov/member/eli-crane/C001135"}'),
('00000003-0002-0003-0000-000000000001', 'Person', 1, '{"name":"Ruben Gallego","role":"U.S. Representative","party":"Democratic","state":"Arizona","district":3,"in_office_since":"2015-01-03","fec_candidate_id":"H4AZ07024","source_url":"https://www.congress.gov/member/ruben-gallego/G000574"}'),
('00000003-0002-0004-0000-000000000001', 'Person', 1, '{"name":"Greg Stanton","role":"U.S. Representative","party":"Democratic","state":"Arizona","district":4,"in_office_since":"2019-01-03","fec_candidate_id":"H8AZ09109","source_url":"https://www.congress.gov/member/greg-stanton/S001211"}'),
('00000003-0002-0005-0000-000000000001', 'Person', 1, '{"name":"Andy Biggs","role":"U.S. Representative","party":"Republican","state":"Arizona","district":5,"in_office_since":"2017-01-03","fec_candidate_id":"H6AZ05064","source_url":"https://www.congress.gov/member/andy-biggs/B001302"}'),
('00000003-0002-0006-0000-000000000001', 'Person', 1, '{"name":"Juan Ciscomani","role":"U.S. Representative","party":"Republican","state":"Arizona","district":6,"in_office_since":"2023-01-03","fec_candidate_id":"H2AZ06132","source_url":"https://www.congress.gov/member/juan-ciscomani/C001133"}'),
('00000003-0002-0007-0000-000000000001', 'Person', 1, '{"name":"Raúl Grijalva","role":"U.S. Representative","party":"Democratic","state":"Arizona","district":7,"in_office_since":"2003-01-03","fec_candidate_id":"H2AZ07040","source_url":"https://www.congress.gov/member/raul-grijalva/G000551"}'),
('00000003-0002-0008-0000-000000000001', 'Person', 1, '{"name":"Debbie Lesko","role":"U.S. Representative","party":"Republican","state":"Arizona","district":8,"in_office_since":"2018-04-24","fec_candidate_id":"H8AZ08027","source_url":"https://www.congress.gov/member/debbie-lesko/L000589"}'),
('00000003-0002-0009-0000-000000000001', 'Person', 1, '{"name":"Paul Gosar","role":"U.S. Representative","party":"Republican","state":"Arizona","district":9,"in_office_since":"2011-01-03","fec_candidate_id":"H0AZ01259","source_url":"https://www.congress.gov/member/paul-gosar/G000565"}');

-- ════════════════════════════════════════════════════════════════════════════
-- KEY ORGANIZATIONS
-- ════════════════════════════════════════════════════════════════════════════

INSERT OR IGNORE INTO entities (id, entity_type, version, data) VALUES
('00000004-0000-0000-0000-000000000001', 'Organization', 1, '{"name":"Democratic National Committee","abbreviation":"DNC","org_type":"Political Party Committee","fec_committee_id":"C00010603","source_url":"https://www.fec.gov/data/committee/C00010603/"}'),
('00000004-0000-0000-0000-000000000002', 'Organization', 1, '{"name":"Republican National Committee","abbreviation":"RNC","org_type":"Political Party Committee","fec_committee_id":"C00003418","source_url":"https://www.fec.gov/data/committee/C00003418/"}'),
('00000004-0000-0000-0000-000000000003', 'Organization', 1, '{"name":"National Rifle Association","abbreviation":"NRA","org_type":"Lobbying Organization","source_url":"https://www.opensecrets.org/orgs/national-rifle-assn/summary?id=D000000082"}'),
('00000004-0000-0000-0000-000000000004', 'Organization', 1, '{"name":"Planned Parenthood","org_type":"Nonprofit / Political Action","source_url":"https://www.opensecrets.org/orgs/planned-parenthood/summary?id=D000000591"}'),
('00000004-0000-0000-0000-000000000005', 'Organization', 1, '{"name":"American Israel Public Affairs Committee","abbreviation":"AIPAC","org_type":"Lobbying Organization","source_url":"https://www.opensecrets.org/orgs/aipac/summary?id=D000046963"}'),
('00000004-0000-0000-0000-000000000006', 'Organization', 1, '{"name":"Koch Industries","org_type":"Corporation","source_url":"https://www.opensecrets.org/orgs/koch-industries/summary?id=D000000186"}'),
('00000004-0000-0000-0000-000000000007', 'Organization', 1, '{"name":"U.S. Chamber of Commerce","org_type":"Trade Association / Lobbying","source_url":"https://www.opensecrets.org/orgs/us-chamber-of-commerce/summary?id=D000019798"}'),
('00000004-0000-0000-0000-000000000008', 'Organization', 1, '{"name":"Arizona Republican Party","org_type":"State Party Committee","source_url":"https://azgop.org/"}'),
('00000004-0000-0000-0000-000000000009', 'Organization', 1, '{"name":"Arizona Democratic Party","org_type":"State Party Committee","source_url":"https://azdem.org/"}');

-- ════════════════════════════════════════════════════════════════════════════
-- SENATE LEADERSHIP (119th Congress)
-- ════════════════════════════════════════════════════════════════════════════

INSERT OR IGNORE INTO entities (id, entity_type, version, data) VALUES
('00000003-0001-0000-0000-000000000100', 'Person', 1, '{"name":"Chuck Schumer","role":"Senate Majority Leader","party":"Democratic","state":"New York","in_office_since":"2017-01-03","fec_candidate_id":"S8NY00082","source_url":"https://www.congress.gov/member/charles-schumer/S000148"}'),
('00000003-0001-0000-0000-000000000101', 'Person', 1, '{"name":"Mitch McConnell","role":"Senate Minority Leader","party":"Republican","state":"Kentucky","in_office_since":"2015-01-03","fec_candidate_id":"S2KY00012","source_url":"https://www.congress.gov/member/mitch-mcconnell/M000355"}'),
('00000003-0001-0000-0000-000000000102', 'Person', 1, '{"name":"Dick Durbin","role":"Senate Majority Whip","party":"Democratic","state":"Illinois","in_office_since":"2005-01-04","fec_candidate_id":"S6IL00151","source_url":"https://www.congress.gov/member/richard-durbin/D000563"}'),
('00000003-0001-0000-0000-000000000103', 'Person', 1, '{"name":"John Thune","role":"Senate Minority Whip","party":"Republican","state":"South Dakota","in_office_since":"2019-01-03","fec_candidate_id":"S4SD00049","source_url":"https://www.congress.gov/member/john-thune/T000250"}');

-- ════════════════════════════════════════════════════════════════════════════
-- HOUSE LEADERSHIP (119th Congress)
-- ════════════════════════════════════════════════════════════════════════════

INSERT OR IGNORE INTO entities (id, entity_type, version, data) VALUES
('00000003-0002-0000-0000-000000000100', 'Person', 1, '{"name":"Mike Johnson","role":"Speaker of the House","party":"Republican","state":"Louisiana","district":4,"in_office_since":"2023-10-25","fec_candidate_id":"H6LA04140","source_url":"https://www.congress.gov/member/mike-johnson/J000299"}'),
('00000003-0002-0000-0000-000000000101', 'Person', 1, '{"name":"Hakeem Jeffries","role":"House Minority Leader","party":"Democratic","state":"New York","district":8,"in_office_since":"2023-01-03","fec_candidate_id":"H2NY10064","source_url":"https://www.congress.gov/member/hakeem-jeffries/J000294"}'),
('00000003-0002-0000-0000-000000000102', 'Person', 1, '{"name":"Steve Scalise","role":"House Majority Leader","party":"Republican","state":"Louisiana","district":1,"in_office_since":"2023-01-03","fec_candidate_id":"H8LA01052","source_url":"https://www.congress.gov/member/steve-scalise/S001176"}'),
('00000003-0002-0000-0000-000000000103', 'Person', 1, '{"name":"Katherine Clark","role":"House Minority Whip","party":"Democratic","state":"Massachusetts","district":5,"in_office_since":"2023-01-03","fec_candidate_id":"H4MA05093","source_url":"https://www.congress.gov/member/katherine-clark/C001101"}');

-- ════════════════════════════════════════════════════════════════════════════
-- RELATIONSHIPS
-- ════════════════════════════════════════════════════════════════════════════

-- Party memberships
INSERT OR IGNORE INTO relationships (id, from_entity, to_entity, rel_type, data) VALUES
('10000001-0000-0000-0000-000000000001', '00000001-0000-0000-0000-000000000001', '00000004-0000-0000-0000-000000000001', 'MemberOf', '{"description":"Biden is a member of the Democratic Party"}'),
('10000001-0000-0000-0000-000000000002', '00000001-0000-0000-0000-000000000002', '00000004-0000-0000-0000-000000000001', 'MemberOf', '{"description":"Harris is a member of the Democratic Party"}'),
('10000001-0000-0000-0000-000000000003', '00000003-0001-0000-0000-000000000001', '00000004-0000-0000-0000-000000000001', 'MemberOf', '{"description":"Sinema formerly Democratic, now Independent"}'),
('10000001-0000-0000-0000-000000000004', '00000003-0001-0000-0000-000000000002', '00000004-0000-0000-0000-000000000001', 'MemberOf', '{"description":"Kelly is a member of the Democratic Party"}'),
('10000001-0000-0000-0000-000000000005', '00000003-0002-0001-0000-000000000001', '00000004-0000-0000-0000-000000000002', 'MemberOf', '{"description":"Schweikert is a member of the Republican Party"}'),
('10000001-0000-0000-0000-000000000006', '00000003-0002-0005-0000-000000000001', '00000004-0000-0000-0000-000000000002', 'MemberOf', '{"description":"Biggs is a member of the Republican Party"}'),
('10000001-0000-0000-0000-000000000007', '00000003-0002-0009-0000-000000000001', '00000004-0000-0000-0000-000000000002', 'MemberOf', '{"description":"Gosar is a member of the Republican Party"}');

-- Appointment relationships
INSERT OR IGNORE INTO relationships (id, from_entity, to_entity, rel_type, data) VALUES
('10000002-0000-0000-0000-000000000001', '00000001-0000-0000-0000-000000000001', '00000001-0000-0000-0000-000000000010', 'Appointed', '{"description":"Biden appointed Blinken as Secretary of State","date":"2021-01-26"}'),
('10000002-0000-0000-0000-000000000002', '00000001-0000-0000-0000-000000000001', '00000001-0000-0000-0000-000000000011', 'Appointed', '{"description":"Biden appointed Yellen as Secretary of Treasury","date":"2021-01-26"}'),
('10000002-0000-0000-0000-000000000003', '00000001-0000-0000-0000-000000000001', '00000001-0000-0000-0000-000000000012', 'Appointed', '{"description":"Biden appointed Austin as Secretary of Defense","date":"2021-01-22"}'),
('10000002-0000-0000-0000-000000000004', '00000001-0000-0000-0000-000000000001', '00000001-0000-0000-0000-000000000013', 'Appointed', '{"description":"Biden appointed Garland as Attorney General","date":"2021-03-11"}'),
('10000002-0000-0000-0000-000000000005', '00000001-0000-0000-0000-000000000001', '00000002-0000-0000-0000-000000000009', 'Appointed', '{"description":"Biden appointed Jackson to Supreme Court","date":"2022-06-30"}');
