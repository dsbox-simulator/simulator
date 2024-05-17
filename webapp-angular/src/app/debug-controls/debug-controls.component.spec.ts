import { ComponentFixture, TestBed } from '@angular/core/testing';

import { DebugControlsComponent } from './debug-controls.component';

describe('DebugControlsComponent', () => {
  let component: DebugControlsComponent;
  let fixture: ComponentFixture<DebugControlsComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [DebugControlsComponent]
    })
    .compileComponents();
    
    fixture = TestBed.createComponent(DebugControlsComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
