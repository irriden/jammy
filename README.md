
### Approach
In our approach, we decided to 
- Acquire reputation to get our htlcs endorsed across the target's channels.
- Probe the total outbound liquidity on each of the target's channels with endorsed htlcs.
- Execute a fast jamming attack - where the size of each htlc is the total outbound liquidity on the channel,
and each htlc is never held for more than 90 seconds.
- During the attack the htlcs remain endorsed, and repeatedly lock up the full amount of the outbound liquidty on the channel.
- The target is unable to route any other payments.

### Setup
In our setup, we had two nodes
- nodeA connecting to the target node (channel should be the same size of all of nodeB's channels)
- nodeB connecting to all of the target node's peers (should be equal or greater to channel size of target to peer)

##### Diagram
<img width="687" alt="image" src="https://github.com/irriden/jammy/assets/15950706/8a0e210d-f3ac-4009-95bb-46696e6fd56a">

### Implementation
We have two loops in our code, one to gain the endorsement and one to execute the attack

### Estimated cost
In this attack, the estimated cost will be </br>
1). The fees and liquidity needed to open 1 channel to the target plus a channel for each peer </br>
2). The cost of getting the payment to be endorsed </br>
3). The upfront fees since we will be canceling the transaction and sending another one after less than 90 seconds </br>

we used the idea that we don't lose reputation if we resolve before the `resolution_period` in this attack
