import { EventPipe } from './event.pipe';

describe('EventPipe', () => {
  it('create an instance', () => {
    const pipe = new EventPipe();
    expect(pipe).toBeTruthy();
  });
});
