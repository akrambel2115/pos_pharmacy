export function formatDateFr(dateStr: string | null | undefined): string {
  if (!dateStr) return "-";
  const match = dateStr.match(/^(\d{4})-(\d{2})-(\d{2})(.*)$/);
  if (match) {
    const [_, year, month, day, rest] = match;
    return `${day}-${month}-${year}${rest.trim() ? " " + rest.trim() : ""}`;
  }
  return dateStr;
}

export interface TreatmentStatus {
  daysLeft: number;
  isPassed: boolean;
}

export function calculateRemainingTreatmentDays(
  createdAt: string | null | undefined,
  periodDays: number | null | undefined
): TreatmentStatus | null {
  if (!createdAt || !periodDays || periodDays <= 0) {
    return null;
  }

  const normalized = createdAt.replace(" ", "T");
  const created = new Date(normalized);
  if (isNaN(created.getTime())) {
    return null;
  }

  const now = new Date();
  const createdMidnight = new Date(created.getFullYear(), created.getMonth(), created.getDate());
  const nowMidnight = new Date(now.getFullYear(), now.getMonth(), now.getDate());

  const elapsedDays = Math.round((nowMidnight.getTime() - createdMidnight.getTime()) / (1000 * 60 * 60 * 24));
  const daysLeft = periodDays - elapsedDays;

  return {
    daysLeft,
    isPassed: daysLeft <= 0,
  };
}
