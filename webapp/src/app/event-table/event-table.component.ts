import { Component, OnInit, OnDestroy, Input, OnChanges, SimpleChanges } from '@angular/core';
import { EventStore } from '../models/EventStore';
import { Subscription } from 'rxjs';
import { CommonModule } from '@angular/common';
import { JsonRpcEvent } from '../models/communication/RpcEvent';
import { FormsModule } from '@angular/forms';
import { EventPipe } from './event.pipe';
import { CoreSocketFactory } from '../models/communication/CoreSocketFactory';
import { HighlightJsonPipe } from './json-highlight.pipe';

@Component({
  selector: 'app-event-table',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    EventPipe,
    HighlightJsonPipe
  ],
  templateUrl: './event-table.component.html',
  styleUrls: ['./event-table.component.scss']
})

/**
 * EventTableComponent is a component that displays all events or events not yet delivered in a table.
 */
export class EventTableComponent implements OnInit, OnDestroy, OnChanges {

  //delivered = true means that the table will show only the events that have not been delivered yet
  @Input() delivered: boolean = false;
  public events: JsonRpcEvent[] = EventStore.events;
  private eventsSub!: Subscription;
  filter: string = '';

  public searchText = '';
  public sortKey = '';

  formatToggle: { [key: number]: boolean } = {};

  constructor() { }

  ngOnChanges(changes: SimpleChanges) {
    if (changes['delivered']) {
      this.updateEvents();
    }
  }

  ngOnInit() {
    this.eventsSub = EventStore.eventsUpdated.subscribe((event: JsonRpcEvent) => {
      this.updateEvents();
    });
  }

  ngOnDestroy() {
    this.eventsSub.unsubscribe();
  }

  private updateEvents() {
    if(this.delivered){
      this.events = EventStore.getNonDeliveredMessages();
    }else{
      this.events = EventStore.events;
    }
  }

  /**
   * 
   * @param event Delivers the chosen Message
   */
  deliverEvent(event: JsonRpcEvent) {
    CoreSocketFactory.create().call('deliver', [event.params.timestamp.logical]);
  }

  /**
   * 
   * @param event Drops the chosen Message
   */
  dropEvent(event: JsonRpcEvent) {
    CoreSocketFactory.create().call('drop', [event.params.timestamp.logical]);
    EventStore.dropEvent(event);
    this.updateEvents();
  }

  /**
   * 
   * @param timestamp the timestamp of the event
   * Onclick toggles if JSON should be formatted or not
   */
  toggleFormat(timestamp: number) {
    if (this.formatToggle[timestamp] === undefined) {
      this.formatToggle[timestamp] = true;
    } else {
      this.formatToggle[timestamp] = !this.formatToggle[timestamp];
    }
  }

  isFormatted(timestamp: number): boolean {
    return this.formatToggle[timestamp] || false;
  }

  copyToClipboard(event: JsonRpcEvent) {
    const jsonContent = JSON.stringify(event.params.data, null, 2); // Pretty print JSON
    navigator.clipboard.writeText(jsonContent).then(() => {
      console.log('Copied to clipboard!');
    }).catch(err => {
      console.error('Could not copy text: ', err);
    });
  }
}
