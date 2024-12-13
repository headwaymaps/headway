import { expect, test } from '@jest/globals';
import OpeningHours from './OpeningHours';
import { formatTime } from '../utils/format';

const sundayMorning = new Date('2012/11/11 8:00 AM');
const mondayMorning = new Date('2012/11/12 8:00 AM');
const tuesdayMorning = new Date('2012/11/13 8:00 AM');

test('is open', () => {
  const osmText = 'Su-Th 08:00-21:00; Fr-Sa 08:00-22:00';
  let hours = OpeningHours.fromOsmString(
    osmText,
    new Date('2012/11/11 7:59 AM'),
  );
  expect(hours.isOpen).toBe(false);

  hours = OpeningHours.fromOsmString(osmText, new Date('2012/11/11 8:00 AM'));
  expect(hours.isOpen).toBe(true);

  hours = OpeningHours.fromOsmString(osmText, new Date('2012/11/11 8:59 PM'));
  expect(hours.isOpen).toBe(true);

  hours = OpeningHours.fromOsmString(osmText, new Date('2012/11/11 9:00 PM'));
  expect(hours.isOpen).toBe(false);
});

test('weeklyRanges', () => {
  const hours = OpeningHours.fromOsmString(
    'Su-Th 08:00-21:00; Fr-Sa 08:00-22:00',
    sundayMorning,
  );
  expect(hours.nextChange!.getHours()).toBe(21);

  const ranges = hours.weeklyRanges();
  expect(ranges.length).toBe(7);

  const days = ranges.map((dayInterval) => dayInterval.day);
  expect(days).toEqual(['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']);

  for (const dayInterval of ranges) {
    expect(dayInterval.intervals.length).toBe(1);
    const interval = dayInterval.intervals[0]!;
    switch (dayInterval.day) {
      case 'Sun':
      case 'Mon':
      case 'Tue':
      case 'Wed':
      case 'Thu':
        expect(formatTime(interval[0])).toBe('8:00 AM');
        expect(formatTime(interval[1])).toBe('9:00 PM');
        break;
      case 'Fri':
      case 'Sat':
        expect(formatTime(interval[0])).toBe('8:00 AM');
        expect(formatTime(interval[1])).toBe('10:00 PM');
        break;
      default:
        throw new Error(`unexpected day ${dayInterval.day}`);
    }
  }
});

test('nextChange - closed today and tomorrow', () => {
  const osmText = 'Su-Mo off; Tu-Sa 11:00-15:00,17:00-21:00';
  const hours = OpeningHours.fromOsmString(osmText, sundayMorning);
  expect(hours.nextChangeIsToday!).toBe(false);
  expect(hours.nextChangeIsTomorrow!).toBe(false);
  expect(hours.nextChange!.getHours()).toBe(11);
});

test('nextChange - closed until tomorrow morning', () => {
  const osmText = 'Su-Mo off; Tu-Sa 11:00-15:00,17:00-21:00';
  const hours = OpeningHours.fromOsmString(osmText, mondayMorning);
  expect(hours.nextChangeIsToday!).toBe(false);
  expect(hours.nextChangeIsTomorrow!).toBe(true);
  expect(hours.nextChange!.getHours()).toBe(11);
});

test('nextChange - closed until later this morning', () => {
  const osmText = 'Su-Mo off; Tu-Sa 11:00-15:00,17:00-21:00';
  const hours = OpeningHours.fromOsmString(osmText, tuesdayMorning);
  expect(hours.nextChangeIsToday!).toBe(true);
  expect(hours.nextChangeIsTomorrow!).toBe(false);
  expect(hours.nextChange!.getHours()).toBe(11);
});
