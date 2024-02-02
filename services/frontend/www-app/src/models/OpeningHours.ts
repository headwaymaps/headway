import ParsedOpeningHours from 'opening_hours';

export default class OpeningHours {
  parsed: ParsedOpeningHours;
  now: Date;
  isOpen: boolean;
  nextChange?: Date;

  constructor(parsed: ParsedOpeningHours, now: Date) {
    this.parsed = parsed;
    this.now = now;
    const statePair = parsed.getStatePair(now);
    this.isOpen = statePair[0];

    let stepIdx = 0;
    let prevStatePair = statePair;
    let nextStatePair = parsed.getStatePair(prevStatePair[1]);
    while (nextStatePair[0] == this.isOpen && stepIdx < 10) {
      stepIdx++;
      prevStatePair = nextStatePair;
      nextStatePair = parsed.getStatePair(prevStatePair[1]);
    }
    if (stepIdx == 10) {
      console.error('found seemingly unchanging opening hours');
    } else {
      this.nextChange = prevStatePair[1];
    }
  }

  static fromOsmString(osmString: string, now: Date): OpeningHours {
    const parsed = new ParsedOpeningHours(osmString);
    return new OpeningHours(parsed, now);
  }

  weeklyRanges(): DayInterval[] {
    const days: DayInterval[] = [];
    for (let dayIdx = 0; dayIdx < 7; dayIdx++) {
      const startOfDay = new Date(this.now);
      startOfDay.setHours(0, 0, 0, 0);
      startOfDay.setDate(startOfDay.getDate() + dayIdx);

      const endOfDay = new Date(startOfDay);
      endOfDay.setHours(23, 59, 59, 999);
      const dayName = startOfDay.toLocaleString([], { weekday: 'short' });
      const intervals: Interval[] = this.parsed.getOpenIntervals(
        startOfDay,
        endOfDay,
      );

      days.push({ day: dayName, intervals });
    }
    return days;
  }

  get nextChangeIsToday(): boolean | undefined {
    if (this.nextChange) {
      return this.nextChange.getDate() == this.now.getDate();
    }
  }

  get nextChangeIsTomorrow(): boolean | undefined {
    if (this.nextChange) {
      return this.nextChange.getDate() - this.now.getDate() == 1;
    }
  }
}

type Interval = [Date, Date, boolean, string | undefined];

type DayInterval = { day: string; intervals: Interval[] };
