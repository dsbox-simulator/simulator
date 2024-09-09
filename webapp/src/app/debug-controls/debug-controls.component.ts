import { Component } from '@angular/core';
import { CoreSocketFactory } from '../models/communication/CoreSocketFactory';
import { JsonRpcWebSocketClient } from '../models/communication/RpcSocket';
import { FormsModule } from '@angular/forms';
import { PredicateStore } from '../json-predicate/Models/PredicateStore';
import { LinkedPredicate } from '../json-predicate/Models/LinkedPredicate';
import { EventStore } from '../models/EventStore';
import { NotificationService } from '../notification.service';
import { ConfigurationStore } from '../configurationStore';

@Component({
  selector: 'app-debug-controls',
  standalone: true,
  imports: [FormsModule],
  templateUrl: './debug-controls.component.html',
  styleUrls: ['./debug-controls.component.css'] // Fix typo from 'styleUrl' to 'styleUrls'
})
export class DebugControlsComponent {
  Break: boolean = false;
  StepTime: number = ConfigurationStore.stepTime;

  constructor(private notificationService: NotificationService) {}

  public onChangeStepTime() {
    ConfigurationStore.stepTime = this.StepTime;
    ConfigurationStore.saveConfiguration();
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
    const jsonInTransit = EventStore.getNonDeliveredDsMessages().map((message) => {
      return JSON.parse(JSON.stringify(message.sendMessage.params));
    });

    console.log('Json in transit:', jsonInTransit);

    PredicateStore.getEvents().forEach((predicate: LinkedPredicate, i: number) => {
      if (predicate.endState === true) {
          return;
      }
      const result = predicate.evaluate(jsonInTransit);
      PredicateStore.updateEvent(i, predicate);
      if (result === true) {
          this.Break = true;
          this.notificationService.showNotification('Break condition was met.');
      }
    });
  }

  public break() {
    this.Break = true;
    ConfigurationStore.loadConfiguration();
  }
}
