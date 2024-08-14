import { Component } from '@angular/core';
import { CoreSocketFactory } from '../models/communication/CoreSocketFactory';
import { JsonRpcWebSocketClient } from '../models/communication/RpcSocket';
import { FormsModule } from '@angular/forms';
import { PredicateStore } from '../json-predicate/Models/PredicateStore';
import { LinkedPredicate } from '../json-predicate/Models/LinkedPredicate';
import { EventStore } from '../models/EventStore';

@Component({
  selector: 'app-debug-controls',
  standalone: true,
  imports: [FormsModule],
  templateUrl: './debug-controls.component.html',
  styleUrl: './debug-controls.component.css'
})
export class DebugControlsComponent {
  Break: boolean = false;
  StepTime: number = 1000;
  constructor() {
  }

  public send(data: string) {
    this.Break = true;
    CoreSocketFactory.create().call(data, []);
    this.checkPredicates();
  }

  public async resume() {
    this.Break = false;
    while (!this.Break) {
      CoreSocketFactory.create().call('step', []);
      this.checkPredicates();
     
      await new Promise(resolve => setTimeout(resolve, this.StepTime));
    }
  }

  private checkPredicates() {
     //Check Break condition
     const jsonInTransit = EventStore.getNonDeliveredDsMessages().map((message) => {
      //JSON.stringify(message.sendMessage.params);
      return JSON.parse(JSON.stringify(message.sendMessage.params));
   });
   console.log('Json in transit:', jsonInTransit);
   PredicateStore.getEvents().forEach((predicate: LinkedPredicate) => {
    if (predicate.endState === true) {
        // Skip this predicate if endState is true
        return;
    }
    const result = predicate.evaluate(jsonInTransit);
    if (result === true) {
        this.Break = true;

        alert('Break condition was met.');
    }
  });

  }

  public break() {
    this.Break = true;
  }
}
