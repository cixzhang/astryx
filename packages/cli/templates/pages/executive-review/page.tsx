// Copyright (c) Meta Platforms, Inc. and affiliates.

'use client';

/**
 * Executive / Weekly Business Review — a leadership scorecard dashboard.
 *
 * Content-only (root `Layout`); the host supplies the app shell. The page has
 * two modes, driven by the header's "One-pager" toggle:
 *
 *   Full mode:     scorecard row | OKR attainment | 2x2 trend grid | narrative
 *   One-pager mode: condensed single-column summary sized to read/print as one
 *                   page — smaller charts, tighter spacing, callouts inline.
 *
 * The period control (WoW / MoM / QoQ) reshapes every scorecard delta, the OKR
 * targets, the trend charts, and the auto-generated narrative. All data is
 * deterministic (fixed fixtures, no clocks/random) so previews stay stable.
 */

import {useMemo, useState, type CSSProperties} from 'react';
import {
  VStack,
  HStack,
  StackItem,
  Layout,
  LayoutContent,
  LayoutHeader,
} from '@astryxdesign/core/Layout';
import {Grid} from '@astryxdesign/core/Grid';
import {Text, Heading} from '@astryxdesign/core/Text';
import {Card} from '@astryxdesign/core/Card';
import {Button} from '@astryxdesign/core/Button';
import {Icon} from '@astryxdesign/core/Icon';
import {Divider} from '@astryxdesign/core/Divider';
import {Badge} from '@astryxdesign/core/Badge';
import {StatusDot} from '@astryxdesign/core/StatusDot';
import type {StatusDotVariant} from '@astryxdesign/core/StatusDot';
import {ProgressBar} from '@astryxdesign/core/ProgressBar';
import {Banner} from '@astryxdesign/core/Banner';
import {
  SegmentedControl,
  SegmentedControlItem,
} from '@astryxdesign/core/SegmentedControl';
import {ToggleButton} from '@astryxdesign/core/ToggleButton';
import {Timestamp} from '@astryxdesign/core/Timestamp';
import {
  AreaChart,
  Area,
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import {
  ArrowUpIcon,
  ArrowDownIcon,
  ArrowRightIcon,
  DocumentArrowDownIcon,
  ArrowDownTrayIcon,
} from '@heroicons/react/24/outline';
import {StopIcon} from '@heroicons/react/24/solid';

// ============= TYPES =============

type Period = 'WoW' | 'MoM' | 'QoQ';
type Rag = 'green' | 'amber' | 'red';

interface Kpi {
  key: string;
  label: string;
  value: string;
  unit?: string;
  // Delta percentage per period; sign drives arrow + RAG.
  delta: Record<Period, number>;
  // Whether an increase is good (revenue) or bad (churn, cost).
  higherIsBetter: boolean;
}

interface Okr {
  objective: string;
  owner: string;
  actual: number;
  target: number;
  unit: string;
}

interface TrendSeries {
  key: string;
  title: string;
  unit: string;
  kind: 'area' | 'line';
  color: string;
  // 12 periods of [current, prior] pairs.
  data: {t: number; label: string; current: number; prior: number}[];
}

// ============= RAG COLOR MAPPING =============

const RAG_DOT: Record<Rag, StatusDotVariant> = {
  green: 'success',
  amber: 'warning',
  red: 'error',
};

const RAG_BADGE: Record<Rag, 'green' | 'yellow' | 'red'> = {
  green: 'green',
  amber: 'yellow',
  red: 'red',
};

const RAG_LABEL: Record<Rag, string> = {
  green: 'On track',
  amber: 'At risk',
  red: 'Off track',
};

// A delta becomes RAG by magnitude + whether the direction is favorable.
function ragFor(deltaPct: number, higherIsBetter: boolean): Rag {
  const favorable = higherIsBetter ? deltaPct >= 0 : deltaPct <= 0;
  const mag = Math.abs(deltaPct);
  if (favorable) {
    return mag >= 1 ? 'green' : 'amber';
  }
  return mag >= 3 ? 'red' : 'amber';
}

// ============= CHART COLORS (design tokens w/ hex fallbacks) =============

const COLORS = {
  blue: 'var(--color-data-categorical-blue, #0171E3)',
  green: 'var(--color-data-categorical-green, #0B991F)',
  orange: 'var(--color-data-categorical-orange, #EB6E00)',
  purple: 'var(--color-data-categorical-purple, #6B1EFD)',
  prior: 'var(--color-text-secondary, #737373)',
};
const GRID_STROKE = 'var(--color-border, rgba(5, 54, 89, 0.1))';
const AXIS_TICK = {
  fontSize: 'var(--font-size-sm, 12px)',
  fill: 'var(--color-text-secondary, #4E606F)',
};

// ============= SCORECARD DATA =============

const KPIS: Kpi[] = [
  {
    key: 'revenue',
    label: 'Net revenue',
    value: '$4.82M',
    delta: {WoW: 2.4, MoM: 6.1, QoQ: 14.3},
    higherIsBetter: true,
  },
  {
    key: 'arr',
    label: 'ARR',
    value: '$58.1M',
    delta: {WoW: 1.1, MoM: 3.8, QoQ: 11.2},
    higherIsBetter: true,
  },
  {
    key: 'nrr',
    label: 'Net revenue retention',
    value: '112%',
    delta: {WoW: 0.3, MoM: -0.8, QoQ: 2.1},
    higherIsBetter: true,
  },
  {
    key: 'churn',
    label: 'Gross churn',
    value: '1.9%',
    delta: {WoW: 0.4, MoM: 1.6, QoQ: -0.7},
    higherIsBetter: false,
  },
  {
    key: 'cac',
    label: 'CAC payback',
    value: '14.2 mo',
    delta: {WoW: -0.2, MoM: -3.4, QoQ: -8.1},
    higherIsBetter: false,
  },
  {
    key: 'nps',
    label: 'NPS',
    value: '48',
    delta: {WoW: 1.0, MoM: 4.0, QoQ: 6.0},
    higherIsBetter: true,
  },
];

// ============= OKR / GOAL ATTAINMENT =============

const OKRS: Okr[] = [
  {
    objective: 'Reach $60M ARR',
    owner: 'Dana Whitfield · CRO',
    actual: 58.1,
    target: 60,
    unit: 'M',
  },
  {
    objective: 'Expand into 3 new enterprise verticals',
    owner: 'Marcus Lin · VP Sales',
    actual: 2,
    target: 3,
    unit: '',
  },
  {
    objective: 'Ship AI copilot GA',
    owner: 'Priya Raman · VP Product',
    actual: 82,
    target: 100,
    unit: '%',
  },
  {
    objective: 'Improve gross margin to 78%',
    owner: 'Tom Okafor · CFO',
    actual: 75.4,
    target: 78,
    unit: '%',
  },
];

function okrRag(pct: number): Rag {
  if (pct >= 90) {
    return 'green';
  }
  if (pct >= 70) {
    return 'amber';
  }
  return 'red';
}

const RAG_PROGRESS: Record<Rag, 'success' | 'warning' | 'error'> = {
  green: 'success',
  amber: 'warning',
  red: 'error',
};

// ============= TREND DATA (period-over-period) =============

const WEEK_LABELS = [
  'W1',
  'W2',
  'W3',
  'W4',
  'W5',
  'W6',
  'W7',
  'W8',
  'W9',
  'W10',
  'W11',
  'W12',
];

function makeTrend(
  base: number,
  growth: number,
  priorGap: number,
  wobble: number,
): {t: number; label: string; current: number; prior: number}[] {
  return WEEK_LABELS.map((label, i) => {
    const drift = base * (1 + (growth * i) / 11);
    const wob = Math.sin(i * 1.3) * base * wobble;
    const current = Math.round(drift + wob);
    const prior = Math.round((drift + wob) * (1 - priorGap));
    return {t: i, label, current, prior};
  });
}

const TRENDS: TrendSeries[] = [
  {
    key: 'revenue',
    title: 'Net revenue',
    unit: 'k',
    kind: 'area',
    color: COLORS.blue,
    data: makeTrend(980, 0.24, 0.12, 0.03),
  },
  {
    key: 'pipeline',
    title: 'Qualified pipeline',
    unit: 'k',
    kind: 'area',
    color: COLORS.green,
    data: makeTrend(3200, 0.31, 0.18, 0.05),
  },
  {
    key: 'activation',
    title: 'Activation rate',
    unit: '%',
    kind: 'line',
    color: COLORS.purple,
    data: makeTrend(42, 0.28, 0.09, 0.04),
  },
  {
    key: 'support',
    title: 'Support CSAT',
    unit: '%',
    kind: 'line',
    color: COLORS.orange,
    data: makeTrend(88, 0.06, 0.03, 0.02),
  },
];

// ============= AUTO-GENERATED NARRATIVE =============

interface Callout {
  status: 'success' | 'warning' | 'info';
  title: string;
  detail: string;
}

const NARRATIVE: Record<Period, Callout[]> = {
  WoW: [
    {
      status: 'success',
      title: 'Net revenue up 2.4% week-over-week',
      detail:
        'Driven by three enterprise renewals closing early in the East region; expansion ARR contributed 60% of the lift.',
    },
    {
      status: 'warning',
      title: 'Gross churn ticked up 0.4 points',
      detail:
        'Two mid-market logos churned on price; both flagged in QBRs last month. Save-play in motion for the remaining at-risk cohort.',
    },
    {
      status: 'info',
      title: 'CAC payback holding at 14.2 months',
      detail:
        'Paid acquisition efficiency steady; the shift toward partner-sourced pipeline is starting to show in blended CAC.',
    },
  ],
  MoM: [
    {
      status: 'success',
      title: 'Revenue accelerating: +6.1% month-over-month',
      detail:
        'Second consecutive month of accelerating growth. New-logo bookings and expansion both above plan; net-new pipeline covers 3.2x of next quarter target.',
    },
    {
      status: 'warning',
      title: 'NRR dipped 0.8 points to 112%',
      detail:
        'A seasonal downgrade cycle in the SMB segment pulled retention down slightly. Enterprise NRR remains above 120%.',
    },
    {
      status: 'success',
      title: 'CAC payback improved 3.4 points',
      detail:
        'Sales efficiency gains from the new SDR playbook are compounding; payback now inside the 15-month board target.',
    },
  ],
  QoQ: [
    {
      status: 'success',
      title: 'Strong quarter: revenue +14.3%, ARR +11.2%',
      detail:
        'Best quarter on record for net-new ARR. Two of three new verticals landed anchor customers; the AI copilot beta is converting at 2x the baseline trial rate.',
    },
    {
      status: 'success',
      title: 'Churn down 0.7 points quarter-over-quarter',
      detail:
        'Retention initiatives launched in Q1 are paying off; gross churn at a two-year low and NRR up 2.1 points.',
    },
    {
      status: 'warning',
      title: 'Gross margin at 75.4%, short of the 78% goal',
      detail:
        'Infrastructure cost from the AI copilot GA ramp is the primary drag. Finance projects margin recovery once usage-based pricing lands next quarter.',
    },
  ],
};

const PERIOD_SUBTITLE: Record<Period, string> = {
  WoW: 'Week of Jun 23 – Jun 29, 2026 vs. prior week',
  MoM: 'June 2026 vs. May 2026',
  QoQ: 'Q2 2026 vs. Q1 2026',
};

// ============= SHARED PIECES =============

// Icon's `color` prop only accepts semantic names, but the legend swatch must
// match the exact data-categorical stroke of its chart line — so an inline
// color is required here (same pattern as the shipped dashboard templates).
function LegendDot({color, label}: {color: string; label: string}) {
  const dotStyle: CSSProperties = {color};
  return (
    <HStack gap={2} vAlign="center">
      <Icon icon={StopIcon} size="xsm" style={dotStyle} />
      <Text type="supporting" color="secondary">
        {label}
      </Text>
    </HStack>
  );
}

function DeltaBadge({deltaPct, rag}: {deltaPct: number; rag: Rag}) {
  const flat = deltaPct === 0;
  const arrow = flat
    ? ArrowRightIcon
    : deltaPct > 0
      ? ArrowUpIcon
      : ArrowDownIcon;
  const sign = deltaPct > 0 ? '+' : '';
  return (
    <Badge
      variant={RAG_BADGE[rag]}
      label={`${sign}${deltaPct.toFixed(1)}%`}
      icon={<Icon icon={arrow} size="xsm" color="inherit" />}
    />
  );
}

interface TrendTooltipEntry {
  name: string;
  value: number;
  color: string;
}

function makeTrendTooltip(unit: string) {
  return function TrendTooltip({
    active,
    payload,
    label,
  }: {
    active?: boolean;
    payload?: TrendTooltipEntry[];
    label?: number;
  }) {
    if (!active || !payload?.length) {
      return null;
    }
    const wk =
      typeof label === 'number'
        ? (WEEK_LABELS[label] ?? '')
        : String(label ?? '');
    return (
      <Card padding={3}>
        <VStack gap={1}>
          <Text type="supporting" color="secondary">
            {wk}
          </Text>
          {payload.map(entry => (
            <LegendDot
              key={entry.name}
              color={entry.color}
              label={`${entry.name}: ${entry.value.toLocaleString()}${unit}`}
            />
          ))}
        </VStack>
      </Card>
    );
  };
}

// ============= SCORECARD =============

function ScorecardTile({
  kpi,
  period,
  isCompact,
}: {
  kpi: Kpi;
  period: Period;
  isCompact: boolean;
}) {
  const deltaPct = kpi.delta[period];
  const rag = ragFor(deltaPct, kpi.higherIsBetter);
  return (
    <Card padding={isCompact ? 4 : 5}>
      <VStack gap={isCompact ? 1 : 2}>
        <HStack hAlign="between" vAlign="center">
          <Text type="label" color="secondary">
            {kpi.label}
          </Text>
          <StatusDot
            variant={RAG_DOT[rag]}
            label={RAG_LABEL[rag]}
            tooltip={RAG_LABEL[rag]}
          />
        </HStack>
        <Heading level={isCompact ? 3 : 2}>{kpi.value}</Heading>
        <HStack gap={2} vAlign="center">
          <DeltaBadge deltaPct={deltaPct} rag={rag} />
          <Text type="supporting" color="secondary">
            {period}
          </Text>
        </HStack>
      </VStack>
    </Card>
  );
}

// ============= OKR ATTAINMENT =============

function OkrRow({okr, isCompact}: {okr: Okr; isCompact: boolean}) {
  const pct = Math.round((okr.actual / okr.target) * 100);
  const rag = okrRag(pct);
  return (
    <VStack gap={2}>
      <HStack hAlign="between" vAlign="center" gap={3}>
        <StackItem size="fill">
          <HStack gap={2} vAlign="center">
            <StatusDot variant={RAG_DOT[rag]} label={RAG_LABEL[rag]} />
            <Text type="body" weight="semibold">
              {okr.objective}
            </Text>
          </HStack>
        </StackItem>
        <Badge variant={RAG_BADGE[rag]} label={`${pct}%`} />
      </HStack>
      <ProgressBar
        value={okr.actual}
        max={okr.target}
        variant={RAG_PROGRESS[rag]}
        label={okr.objective}
        isLabelHidden
      />
      {!isCompact && (
        <HStack hAlign="between" vAlign="center">
          <Text type="supporting" color="secondary">
            {okr.owner}
          </Text>
          <Text type="supporting" color="secondary">
            {okr.actual}
            {okr.unit} / {okr.target}
            {okr.unit} target
          </Text>
        </HStack>
      )}
    </VStack>
  );
}

// ============= TREND CHART =============

function TrendChart({trend, height}: {trend: TrendSeries; height: number}) {
  const gradientId = `grad-${trend.key}`;
  const TrendTooltip = useMemo(
    () => makeTrendTooltip(trend.unit),
    [trend.unit],
  );
  const latest = trend.data[trend.data.length - 1];
  const deltaPct = Math.round(
    ((latest.current - latest.prior) / latest.prior) * 100,
  );
  return (
    <Card>
      <VStack gap={3}>
        <HStack hAlign="between" vAlign="center">
          <VStack gap={0}>
            <Text type="label" color="secondary">
              {trend.title}
            </Text>
            <Heading level={4}>
              {latest.current.toLocaleString()}
              {trend.unit}
            </Heading>
          </VStack>
          <Badge
            variant={deltaPct >= 0 ? 'green' : 'red'}
            label={`${deltaPct >= 0 ? '+' : ''}${deltaPct}% vs prior`}
          />
        </HStack>
        <ResponsiveContainer width="100%" height={height}>
          {trend.kind === 'area' ? (
            <AreaChart
              data={trend.data}
              margin={{top: 5, right: 8, left: 0, bottom: 0}}>
              <defs>
                <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor={trend.color} stopOpacity={0.3} />
                  <stop
                    offset="95%"
                    stopColor={trend.color}
                    stopOpacity={0.04}
                  />
                </linearGradient>
              </defs>
              <CartesianGrid horizontal vertical={false} stroke={GRID_STROKE} />
              <XAxis
                dataKey="t"
                type="number"
                domain={[0, 11]}
                ticks={[0, 3, 7, 11]}
                tickFormatter={(v: number) => WEEK_LABELS[v] ?? ''}
                tick={AXIS_TICK}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                tick={AXIS_TICK}
                axisLine={false}
                tickLine={false}
                width={40}
              />
              <Tooltip
                content={<TrendTooltip />}
                cursor={{stroke: GRID_STROKE}}
              />
              <Area
                type="monotone"
                dataKey="prior"
                name="Prior"
                stroke={COLORS.prior}
                strokeWidth={1}
                strokeDasharray="4 4"
                fill="none"
                dot={false}
                isAnimationActive={false}
              />
              <Area
                type="monotone"
                dataKey="current"
                name="Current"
                stroke={trend.color}
                strokeWidth={2}
                fill={`url(#${gradientId})`}
                dot={false}
                isAnimationActive={false}
              />
            </AreaChart>
          ) : (
            <LineChart
              data={trend.data}
              margin={{top: 5, right: 8, left: 0, bottom: 0}}>
              <CartesianGrid horizontal vertical={false} stroke={GRID_STROKE} />
              <XAxis
                dataKey="t"
                type="number"
                domain={[0, 11]}
                ticks={[0, 3, 7, 11]}
                tickFormatter={(v: number) => WEEK_LABELS[v] ?? ''}
                tick={AXIS_TICK}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                tick={AXIS_TICK}
                axisLine={false}
                tickLine={false}
                width={40}
              />
              <Tooltip
                content={<TrendTooltip />}
                cursor={{stroke: GRID_STROKE}}
              />
              <Line
                type="monotone"
                dataKey="prior"
                name="Prior"
                stroke={COLORS.prior}
                strokeWidth={1}
                strokeDasharray="4 4"
                dot={false}
                isAnimationActive={false}
              />
              <Line
                type="monotone"
                dataKey="current"
                name="Current"
                stroke={trend.color}
                strokeWidth={2}
                dot={false}
                isAnimationActive={false}
              />
            </LineChart>
          )}
        </ResponsiveContainer>
        <HStack gap={5} vAlign="center">
          <LegendDot color={trend.color} label="Current" />
          <LegendDot color={COLORS.prior} label="Prior" />
        </HStack>
      </VStack>
    </Card>
  );
}

// ============= NARRATIVE =============

function NarrativeBlock({period}: {period: Period}) {
  return (
    <VStack gap={3}>
      {NARRATIVE[period].map(callout => (
        <Banner
          key={callout.title}
          status={callout.status}
          title={callout.title}
          description={callout.detail}
          container="card"
        />
      ))}
    </VStack>
  );
}

// ============= SECTION HEADING =============

function SectionHeading({title, hint}: {title: string; hint?: string}) {
  return (
    <HStack hAlign="between" vAlign="center" gap={3}>
      <Heading level={3}>{title}</Heading>
      {hint ? (
        <Text type="supporting" color="secondary">
          {hint}
        </Text>
      ) : null}
    </HStack>
  );
}

// ============= MAIN =============

export default function ExecutiveReviewPage() {
  const [period, setPeriod] = useState<Period>('WoW');
  const [onePager, setOnePager] = useState(false);

  const scorecardKpis = onePager ? KPIS.slice(0, 4) : KPIS;
  const chartHeight = onePager ? 120 : 200;
  const stackGap = onePager ? 4 : 6;

  const okrSummary = useMemo(() => {
    const pcts = OKRS.map(o => (o.actual / o.target) * 100);
    const onTrack = pcts.filter(p => p >= 90).length;
    return `${onTrack} of ${OKRS.length} on track`;
  }, []);

  return (
    <Layout
      height="fill"
      contentWidth={onePager ? 900 : 1440}
      header={
        <LayoutHeader hasDivider>
          <HStack gap={3} vAlign="center" wrap="wrap">
            <StackItem size="fill">
              <VStack gap={0}>
                <Heading level={1}>Weekly Business Review</Heading>
                <HStack gap={2} vAlign="center">
                  <Text type="supporting" color="secondary">
                    {PERIOD_SUBTITLE[period]}
                  </Text>
                  <Text type="supporting" color="secondary">
                    · Generated
                  </Text>
                  <Timestamp
                    value="2026-06-30T08:00:00Z"
                    format="date"
                    color="secondary"
                  />
                </HStack>
              </VStack>
            </StackItem>
            <SegmentedControl
              label="Comparison period"
              value={period}
              onChange={value => setPeriod(value as Period)}
              size="sm">
              <SegmentedControlItem label="WoW" value="WoW" />
              <SegmentedControlItem label="MoM" value="MoM" />
              <SegmentedControlItem label="QoQ" value="QoQ" />
            </SegmentedControl>
            <ToggleButton
              label="One-pager"
              icon={<Icon icon={DocumentArrowDownIcon} size="sm" />}
              isPressed={onePager}
              onPressedChange={setOnePager}
              size="sm"
            />
            <Button
              label="Export"
              variant="secondary"
              size="sm"
              icon={<Icon icon={ArrowDownTrayIcon} size="sm" />}
            />
          </HStack>
        </LayoutHeader>
      }
      content={
        <LayoutContent padding={onePager ? 5 : 6}>
          <VStack gap={stackGap}>
            {/* Scorecard row */}
            <VStack gap={3}>
              <SectionHeading
                title="Scorecard"
                hint={`Headline KPIs · ${period} change`}
              />
              <Grid
                columns={{minWidth: onePager ? 200 : 240, repeat: 'fit'}}
                gap={4}>
                {scorecardKpis.map(kpi => (
                  <ScorecardTile
                    key={kpi.key}
                    kpi={kpi}
                    period={period}
                    isCompact={onePager}
                  />
                ))}
              </Grid>
            </VStack>

            {/* One-pager: narrative sits directly under the scorecard so the
                summary reads top-to-bottom on a single page. */}
            {onePager && (
              <VStack gap={3}>
                <SectionHeading title="What changed & why" />
                <NarrativeBlock period={period} />
              </VStack>
            )}

            <Divider />

            {/* OKR attainment */}
            <VStack gap={4}>
              <SectionHeading title="Goal attainment" hint={okrSummary} />
              <Card padding={onePager ? 4 : 6}>
                <VStack gap={onePager ? 4 : 6}>
                  {OKRS.map((okr, i) => (
                    <VStack gap={onePager ? 4 : 6} key={okr.objective}>
                      {i > 0 && <Divider />}
                      <OkrRow okr={okr} isCompact={onePager} />
                    </VStack>
                  ))}
                </VStack>
              </Card>
            </VStack>

            <Divider />

            {/* Trend section: 2x2 grid */}
            <VStack gap={4}>
              <SectionHeading
                title="Trends"
                hint="Current vs. prior period · trailing 12 weeks"
              />
              <Grid
                columns={
                  onePager
                    ? {minWidth: 260, repeat: 'fit'}
                    : {minWidth: 340, repeat: 'fit'}
                }
                gap={4}>
                {TRENDS.map(trend => (
                  <TrendChart
                    key={trend.key}
                    trend={trend}
                    height={chartHeight}
                  />
                ))}
              </Grid>
            </VStack>

            {/* Full mode: narrative closes out the review. */}
            {!onePager && (
              <>
                <Divider />
                <VStack gap={4}>
                  <SectionHeading
                    title="What changed & why"
                    hint="Auto-generated summary"
                  />
                  <NarrativeBlock period={period} />
                </VStack>
              </>
            )}
          </VStack>
        </LayoutContent>
      }
    />
  );
}
