import { ComponentFixture, TestBed } from '@angular/core/testing';

import { GraphInTransitComponent } from './graph-in-transit.component';

describe('GraphInTransitComponent', () => {
  let component: GraphInTransitComponent;
  let fixture: ComponentFixture<GraphInTransitComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [GraphInTransitComponent]
    })
    .compileComponents();
    
    fixture = TestBed.createComponent(GraphInTransitComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
