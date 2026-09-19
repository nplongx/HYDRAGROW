import { useMemo } from 'react';
import type { ReactNode } from 'react';
import { Activity, Bell, Droplets, Leaf, MoreHorizontal, Settings2, Thermometer, Waves, Zap } from 'lucide-react';
import { Link, useSearchParams } from 'react-router-dom';

type ConceptId = 'a' | 'b' | 'c';

const concepts: Record<ConceptId, { name: string; summary: string; focus: string }> = {
  a: { name: 'A — Command Center', summary: 'Một vùng trạng thái lớn, telemetry bento bên dưới.', focus: 'Ưu tiên trạng thái + hành động' },
  b: { name: 'B — Telemetry First', summary: 'Số liệu chiếm ưu thế; cảnh báo và hành động nằm cùng trục.', focus: 'Ưu tiên quan sát nhanh' },
  c: { name: 'C — Farm Overview', summary: 'Bố cục cân bằng giữa trạm, canh tác và hoạt động gần đây.', focus: 'Ưu tiên ngữ cảnh vận hành' },
};

const sensors = [
  { label: 'EC', value: '1.42', unit: 'mS/cm', status: 'Trong ngưỡng', icon: Droplets, tone: 'text-water' },
  { label: 'pH', value: '6.1', unit: '', status: 'Ổn định', icon: Activity, tone: 'text-primary' },
  { label: 'Nhiệt độ', value: '24.8', unit: '°C', status: 'Ổn định', icon: Thermometer, tone: 'text-warning' },
  { label: 'Mực nước', value: '78', unit: '%', status: 'Tốt', icon: Waves, tone: 'text-info-fg' },
];

function StatusPill({ children, tone = 'success' }: { children: ReactNode; tone?: 'success' | 'warning' | 'neutral' }) {
  const classes = tone === 'success'
    ? 'bg-success-bg text-status'
    : tone === 'warning'
      ? 'bg-warning-bg text-warn-deep'
      : 'bg-surface-muted text-text-muted';
  return <span className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-bold ${classes}`}><span className="h-1.5 w-1.5 rounded-full bg-current" />{children}</span>;
}

function Sparkline() {
  const points = [32, 48, 43, 61, 54, 72, 67, 82, 75, 88, 84, 92];
  return (
    <div className="flex h-20 items-end gap-1.5" aria-label="Biểu đồ xu hướng EC 12 giờ">
      {points.map((height, index) => <span key={index} className="flex-1 rounded-t bg-primary/20" style={{ height: `${height}%` }} />)}
    </div>
  );
}

function Sidebar({ concept }: { concept: ConceptId }) {
  return (
    <aside className="hidden min-h-screen w-60 shrink-0 border-r border-line bg-white px-4 py-5 lg:block">
      <Link to="/design-lab?concept=a" className="flex items-center gap-2 px-2">
        <span className="grid h-9 w-9 place-items-center rounded-xl bg-primary-deep text-white"><Leaf size={17} /></span>
        <span><strong className="block text-sm text-primary-deep">HydraGrow</strong><small className="text-[9px] font-bold tracking-[.16em] text-faint">DESIGN LAB</small></span>
      </Link>
      <div className="mt-8 space-y-1">
        {['Tổng quan', 'Vận hành', 'Canh tác', 'Nhật ký', 'Cài đặt'].map((item, index) => (
          <div key={item} className={`flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm ${index === 0 ? 'bg-pill font-bold text-primary-deep' : 'text-text-muted'}`}>
            <span className="h-1.5 w-1.5 rounded-full bg-current" />{item}
          </div>
        ))}
      </div>
      <div className="mt-8 rounded-2xl border border-line bg-surface-muted p-3">
        <p className="text-[10px] font-bold uppercase tracking-wider text-faint">Concept</p>
        <p className="mt-1 text-sm font-bold text-primary-deep">{concepts[concept].name}</p>
        <p className="mt-1 text-xs text-text-muted">{concepts[concept].focus}</p>
      </div>
    </aside>
  );
}

function Header({ concept }: { concept: ConceptId }) {
  return (
    <header className="flex flex-col gap-4 border-b border-line bg-white px-4 py-4 md:px-7 lg:flex-row lg:items-center lg:justify-between">
      <div>
        <p className="text-[10px] font-bold uppercase tracking-[.16em] text-primary">Design Lab · dashboard</p>
        <h1 className="mt-1 text-2xl font-bold tracking-tight text-primary-deep">Tổng quan trạm</h1>
        <p className="text-xs text-text-muted">Fixture data — dùng để so sánh composition, không phải telemetry production.</p>
      </div>
      <div className="flex items-center gap-2">
        <StatusPill>Trạm Online</StatusPill>
        <button className="grid h-9 w-9 place-items-center rounded-xl border border-line bg-white text-text-muted" aria-label="Thông báo"><Bell size={16} /></button>
        <button className="grid h-9 w-9 place-items-center rounded-xl border border-line bg-white text-text-muted" aria-label="Cài đặt"><Settings2 size={16} /></button>
        <span className="hidden text-xs font-bold text-text-muted md:inline">Concept {concept.toUpperCase()}</span>
      </div>
    </header>
  );
}

function SensorGrid() {
  return (
    <section className="grid grid-cols-2 gap-3 xl:grid-cols-4">
      {sensors.map(({ label, value, unit, status, icon: Icon, tone }) => (
        <article key={label} className="rounded-2xl border border-line bg-white p-4 shadow-[0_1px_2px_rgba(20,83,45,0.04)]">
          <div className="flex items-center justify-between"><span className="text-[10px] font-bold uppercase tracking-wider text-faint">{label}</span><Icon size={16} className={tone} /></div>
          <div className="mt-3 flex items-baseline gap-1"><strong className="text-2xl font-black tracking-tight text-primary-deep">{value}</strong><span className="text-xs text-text-muted">{unit}</span></div>
          <p className="mt-1 text-[11px] font-semibold text-status">{status}</p>
        </article>
      ))}
    </section>
  );
}

function ConceptA() {
  return <>
    <section className="grid gap-4 xl:grid-cols-[1.5fr_.8fr]">
      <article className="rounded-3xl border border-line bg-white p-5 md:p-7">
        <div className="flex flex-wrap items-center justify-between gap-3"><StatusPill>Đang vận hành</StatusPill><span className="text-xs font-semibold text-text-muted">Chế độ Tự động</span></div>
        <div className="mt-7 max-w-2xl"><p className="text-[11px] font-bold uppercase tracking-[.15em] text-faint">Trạng thái trạm</p><h2 className="mt-2 text-3xl font-black tracking-tight text-primary-deep md:text-4xl">Mọi hệ thống đang ổn định.</h2><p className="mt-3 max-w-xl text-sm text-text-muted">Không có cảnh báo cần xử lý. Chu trình phun sương tiếp tục theo lịch đã cấu hình.</p></div>
        <div className="mt-7 flex flex-wrap gap-2"><button className="ui-btn-primary">Mở Vận hành</button><button className="ui-btn-outline">Xem lịch hoạt động</button></div>
      </article>
      <article className="rounded-3xl border border-line bg-primary-deep p-5 text-white md:p-7"><p className="text-[11px] font-bold uppercase tracking-[.15em] text-white/60">Sức khỏe trạm</p><div className="mt-6 text-6xl font-black">92<span className="text-xl text-white/50">/100</span></div><p className="mt-2 text-sm text-white/70">Tốt · 4 cảm biến đang đo</p><div className="mt-8 h-2 overflow-hidden rounded-full bg-white/15"><div className="h-full w-[92%] rounded-full bg-white" /></div></article>
    </section>
    <SensorGrid />
    <section className="grid gap-4 lg:grid-cols-[1.4fr_.8fr]"><article className="rounded-2xl border border-line bg-white p-5"><div className="flex items-center justify-between"><div><h3 className="font-bold text-primary-deep">EC · 12 giờ</h3><p className="text-xs text-text-muted">Xu hướng quanh ngưỡng mục tiêu</p></div><MoreHorizontal size={18} className="text-faint" /></div><div className="mt-5"><Sparkline /></div></article><ActivityFeed /></section>
  </>;
}

function ConceptB() {
  return <>
    <SensorGrid />
    <section className="grid gap-4 lg:grid-cols-[1.6fr_.8fr]"><article className="rounded-3xl border border-line bg-white p-5 md:p-7"><div className="flex items-end justify-between"><div><p className="text-[10px] font-bold uppercase tracking-wider text-faint">EC hiện tại</p><div className="mt-2 flex items-baseline gap-2"><strong className="text-5xl font-black text-primary-deep">1.42</strong><span className="text-sm text-text-muted">mS/cm</span></div></div><StatusPill>Trong ngưỡng</StatusPill></div><div className="mt-8"><Sparkline /></div><div className="mt-3 flex justify-between text-[10px] font-semibold text-faint"><span>08:00</span><span>14:00</span><span>20:00</span></div></article><aside className="rounded-3xl border border-warning/30 bg-warning-bg p-5"><p className="text-[10px] font-bold uppercase tracking-wider text-warn-deep">Cần chú ý</p><h3 className="mt-3 text-lg font-bold text-primary-deep">Mực nước giảm nhẹ</h3><p className="mt-2 text-sm text-text-muted">78% — chưa cần can thiệp. Theo dõi chu kỳ cấp nước tiếp theo.</p><button className="ui-btn-outline mt-6 w-full">Mở chi tiết</button></aside></section>
    <ActivityFeed />
  </>;
}

function ConceptC() {
  return <>
    <section className="grid gap-4 lg:grid-cols-[1fr_1fr]"><article className="rounded-3xl border border-line bg-white p-5 md:p-7"><div className="flex items-center gap-2"><span className="grid h-10 w-10 place-items-center rounded-xl bg-pill text-primary"><Leaf size={19} /></span><div><p className="text-[10px] font-bold uppercase tracking-wider text-faint">Khu vực đang trồng</p><h2 className="font-bold text-primary-deep">Rau xà lách · Kệ A</h2></div></div><div className="mt-7 grid grid-cols-3 gap-2"><Metric label="Ngày" value="18" suffix="/ 32" /><Metric label="EC" value="1.42" suffix="mS/cm" /><Metric label="pH" value="6.1" /></div><div className="mt-6 rounded-2xl bg-surface-muted p-4"><div className="flex justify-between text-xs font-semibold text-text-muted"><span>Tiến độ vụ</span><span>56%</span></div><div className="mt-2 h-2 rounded-full bg-white"><div className="h-full w-[56%] rounded-full bg-primary" /></div></div></article><article className="rounded-3xl border border-line bg-white p-5 md:p-7"><div className="flex items-center justify-between"><div><p className="text-[10px] font-bold uppercase tracking-wider text-faint">Hoạt động tiếp theo</p><h2 className="mt-1 font-bold text-primary-deep">Phun sương</h2></div><span className="grid h-10 w-10 place-items-center rounded-xl bg-info-bg text-info-fg"><Zap size={18} /></span></div><div className="mt-6 text-3xl font-black text-primary-deep">02:14 <span className="text-sm font-semibold text-text-muted">nữa</span></div><p className="mt-2 text-xs text-text-muted">Chu kỳ kế tiếp · Tự động · 45 giây</p><button className="ui-btn-primary mt-6 w-full">Mở Vận hành</button></article></section>
    <SensorGrid /><ActivityFeed />
  </>;
}

function Metric({ label, value, suffix }: { label: string; value: string; suffix?: string }) { return <div className="rounded-2xl border border-line bg-surface-muted p-3"><p className="text-[10px] font-bold uppercase tracking-wider text-faint">{label}</p><p className="mt-2 text-lg font-black text-primary-deep">{value} <small className="text-[10px] font-semibold text-text-muted">{suffix}</small></p></div>; }

function ActivityFeed() {
  return <article className="rounded-2xl border border-line bg-white p-5"><div className="flex items-center justify-between"><div><h3 className="font-bold text-primary-deep">Hoạt động gần đây</h3><p className="text-xs text-text-muted">Các sự kiện mới nhất của trạm</p></div><span className="text-xs font-bold text-primary">Xem nhật ký</span></div><div className="mt-5 space-y-4">{[['20:14', 'Chu kỳ phun sương hoàn tất', '45 giây'], ['19:58', 'Đo cảm biến hoàn tất', 'EC · pH · nhiệt độ'], ['19:40', 'Kiểm tra sức khỏe trạm', '92/100']].map(([time, title, detail]) => <div key={time} className="flex gap-3"><div className="w-12 shrink-0 text-[11px] font-bold text-faint">{time}</div><div className="mt-1 h-2 w-2 shrink-0 rounded-full bg-primary" /><div><p className="text-sm font-semibold text-primary-deep">{title}</p><p className="text-xs text-text-muted">{detail}</p></div></div>)}</div></article>;
}

export default function DesignLab() {
  const [params, setParams] = useSearchParams();
  const raw = params.get('concept')?.toLowerCase();
  const concept: ConceptId = raw === 'b' || raw === 'c' ? raw : 'a';
  const content = useMemo(() => concept === 'b' ? <ConceptB /> : concept === 'c' ? <ConceptC /> : <ConceptA />, [concept]);

  const select = (id: ConceptId) => setParams({ concept: id });

  return <div className="min-h-screen bg-surface-muted text-primary-deep lg:flex">
    <Sidebar concept={concept} />
    <div className="min-w-0 flex-1">
      <Header concept={concept} />
      <main className="mx-auto max-w-[1500px] space-y-5 p-4 pb-10 md:p-7">
        <section className="rounded-2xl border border-primary/20 bg-white p-3 shadow-[0_1px_2px_rgba(20,83,45,0.04)]"><div className="flex flex-wrap gap-2">{(Object.keys(concepts) as ConceptId[]).map(id => <button key={id} type="button" onClick={() => select(id)} className={`rounded-xl px-3.5 py-2 text-xs font-bold transition-colors ${id === concept ? 'bg-primary-deep text-white' : 'bg-surface-muted text-text-muted hover:text-primary-deep'}`}>{concepts[id].name}</button>)}<span className="ml-auto hidden items-center text-[11px] font-semibold text-faint md:flex">A/B/C là composition thật · cùng tokens · cùng information model</span></div><p className="mt-2 px-1 text-xs text-text-muted md:hidden">{concepts[concept].summary}</p></section>
        {content}
      </main>
    </div>
  </div>;
}
