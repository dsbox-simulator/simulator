import { Subject } from 'rxjs';
import Event from './Event';

export class EventStore {
  static events: Event[] = [];
  static eventsUpdated = new Subject<Event>();

  static addEvent(event: Event) {
    EventStore.events.push(event);
    EventStore.eventsUpdated.next(event);
  }
}