import { BehaviorSubject } from 'rxjs';
import { LinkedPredicate } from './LinkedPredicate';

/**
 * A store for predicates that can be used to evaluate JSON messages.
 */
export class PredicateStore {
    // The main events array
    private static _events: LinkedPredicate[] = [];

    // Subject to track changes in the events array
    private static eventsSubject = new BehaviorSubject<LinkedPredicate[]>(PredicateStore._events);

    // Observable for other parts of the application to subscribe to
    static eventsChanged = PredicateStore.eventsSubject.asObservable();

    // Method to add a new event
    static addEvent(event: LinkedPredicate) {
        PredicateStore._events.push(event);
        PredicateStore.eventsSubject.next(PredicateStore._events);
    }

    // Method to remove an event by index
    static removeEvent(index: number) {
        PredicateStore._events.splice(index, 1);
        PredicateStore.eventsSubject.next(PredicateStore._events);
    }

    // Method to update an event at a specific index
    static updateEvent(index: number, event: LinkedPredicate) {
        PredicateStore._events[index] = event;
        PredicateStore.eventsSubject.next(PredicateStore._events);
    }

    // Method to get the current list of events
    static getEvents() {
        return PredicateStore._events;
    }
}
