import { Component } from '@angular/core';
import { CoreSocketFactory } from '../models/communication/CoreSocketFactory';
import { JsonRpcWebSocketClient } from '../models/communication/RpcSocket';
import { FormsModule } from '@angular/forms';

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
  }

  public async resume() {
    this.Break = false;
    while (!this.Break) {
      CoreSocketFactory.create().call('step', []);
      await new Promise(resolve => setTimeout(resolve, this.StepTime));
    }
  }

  public break() {
    this.Break = true;
  }
}
