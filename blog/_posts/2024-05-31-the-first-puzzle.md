---
layout: post
title: "The first puzzle"
author: Simon Berger
---

The entire project was kicked off by a short Discord exchange:

{% include figure.html asset="the-first-puzzle/c4dca62b7b5c.png" description="Screenshot of the Discord conversation that started it all." %}

The day of the deadline had already arrived and I was completely unaware. Kevin's suggestion to collaborate on an
entry was very welcome at this point as you might expect.

## The idea

At 21:30 we started a call to design our submission together. The idea was simple: our submission points the players
to a website where the rest of the puzzle takes place. This little hack allows us to essentially extend the deadline
until someone finally tries to solve it.
We quickly came up with the same idea to split up a QR code so you would have to fold the paper correctly to
fully assemble it.

Our initial approach was to just fold the paper randomly, add the QR code to it and then unfold it again.

{% include figure.html asset="the-first-puzzle/59af47b7da12.jpg" description="My poor attempt of creating an interesting pattern." %}

Turns out the result doesn't look very nice though.
We wanted to do a bit better and decided to go for some kind of origami, not yet realizing how much time that would
take.

## First attempt

We remembered that there's this quite popular folding technique called [Himmel und Hölle](https://einfach-basteln.com/himmel-und-hoelle/).
After looking up the instructions Kevin quickly folded one together.

{% include figure.html asset="the-first-puzzle/5de6c1213c74.jpg" description="Kevin's wonderful origami." %}

We wanted to project the QR code onto the paper so that you would have to scan it from above like seen above.
To do this, Kevin added markings to the paper, unfolded it, and then scanned it.

<div class="figure-row">
{% include figure.html
    asset="the-first-puzzle/e64fb54efc99.png"
    description="Markings we added to help with the projection. The violet lines appear straight when it's fully folded and viewed from above."
%}
{% include figure.html
    asset="the-first-puzzle/6b7253c49a55.png"
    description="The paper scan. The area between green and violet is where the content of the QR code goes."
%}
</div>

We rotated and overlaid the 4 corners on top of each other to get an idea of how accurate the lines are. 

{% include figure.html asset="the-first-puzzle/b6ca9d6dc9ae.png" description="All four corners rotated and aligned on the green line." %}

Turns out they're not very accurate...
Hoping for the best we kind of just used the average shape to model a single "wing" and with some copying and rotating
we put together the full projected square.

{% include figure.html asset="the-first-puzzle/1c0bb27cf9ae.png" description="The projection of the square. You can tell how accurate it is based on the very noticeable gaps." %}

We then squeezed a QR code into this shape.
At this point we didn't know what we were going to put on the QR code so it just contained `https://google.com`.

{% include figure.html asset="the-first-puzzle/dc0696216611.png" description="Stretched QR code." %}

All that was left now was to split it back into the 8 wings and rotate them into place.
At this point it was already quite late so figuring out how to rotate the pieces turned out to be much more difficult
than expected and took at least 20 minutes to get right.

{% include figure.html asset="the-first-puzzle/1d1e03d577b5.png" description="The QR code split into its parts and rotated into place." %}

At this point we weren't really convinced that this would work but at least I was still hopeful so we went ahead and
printed it.

<div class="figure-row">
{% include figure.html asset="the-first-puzzle/32925d45eee6.jpg" description="My result." %}
{% include figure.html asset="the-first-puzzle/48b728759eba.jpg" description="Kevin's result." %}
</div>


After folding it neither of us could get our creations to scan. This is probably because we didn't do the projection
correctly or accurately enough. You can clearly tell that the outer lines don't appear straight in mine.
There's also a lot of places where the folds just aren't accurate enough which results in gaps in some places and
overlaps in others.

## Second attempt

It was almost 23:00 and our first attempt completely failed. Instead of spending more time trying to fix it we
decided to go for a simpler design without the final folds that turn it into 3d.
This removes the need for any projection and also makes the folding much more foolproof.

Thanks to our experience with the first attempt we already had a workflow for easily creating the new image.

{% include figure.html asset="the-first-puzzle/cf3a296fd544.png" description="Ready to print." %}

At this point we also knew that we wanted to put a password into this QR code instead of a URL. This would keep the
data short giving us more redundancy.

Again, we both printed and folded the paper and this time it worked for both of us.

<div class="figure-row">
{% include figure.html asset="the-first-puzzle/c33d5608fc36.jpg" description="Kevin's result." %}
{% include figure.html asset="the-first-puzzle/88048431037b.jpg" description="My result." %}
</div>

## Adding the rest

At this point it was already 23:30 so we scrambled to put together the rest of the submission.

We used ChatGPT to come up with an initial premise and added that to the center of the image. The name on the other
hand was suggested by Gemini. After finding and buying a fitting domain name for the game we generated a second QR code
to take the players to the website where they get a hint how to fold the paper correctly to get the password from the
second QR code to proceed.

<div class="figure-row">
{% include figure.html asset="the-first-puzzle/0e68b58cd41c.jpg" description="Kevin's test print in black and white." %}
{% include figure.html asset="the-first-puzzle/8d688a7a5a9a.jpg" description="My final test to make sure that it actually works." %}
</div>

And yes, the ink spot is only there because the paper still looked somewhat empty.
Maybe we'll come up with a reason for it to be there while designing the rest of the game.
