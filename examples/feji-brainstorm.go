// Most likely we won't import stdlib stuff, just wanna be explicit where Im
// getting things from
import stdlib::Item; 

// I prefer using structs over classes. For me, I find structs easier to
// understand (It's just a place where you can store related values) than
// classes (Involves a whole lot of oop stuff that oftentimes makes it confusing
// to use).
struct Pickaxe {
	// In go, this would mean that Pickaxe extends item. This syntax is meh for
	// me, but I don't like the idea of doing `struct Pickaxe extends Item`
	// because of my above reason for doing structs
	Item 
	num strength;
	// I am not implementing a custom mining thing because thats too much work,
	// pretend this would actually do something though :grin:
	num speed;
};


// For constructors, we could make it so that any function with the same name as
// a Struct with a return value of that struct.
//
// idk if i like this or not tbh, just some neat idea i had
func Pickaxe(num strength, num speed, txt name) -> Pickaxe {
	// Remember, constructor for structs is just the struct name. Here we
	// create a new Item struct. No need for a `new` keyword since I believe that derived
	// from C++ where `new` implies memory is being allocated
	Item i = Item()

	// We do NOT need to use item tags, though perhaps it could be a compiler
	// optimization? We don't need them because we can just use the fields of
	// the Pickaxe struct
	// i.setTag("strength", strength)
	// i.setTag("speed", speed)

	i.setName(name)

	// `fmt` function could compile `${}` to %var. I think this could be
	// interesting and could make stuff nicer.
	//
	// Empty ${} function like the variadic args of the c function `printf`
	i.setLore(fmt("Strength: ${strength}\nSpeed: ${speed}\nCost: ${}",
		sqrt(Strength * Speed) // Just some random equation to justify the need for cost 
	))

	return Pickaxe {
		i,
		strength: strength,
		speed: speed,
	}
}


// This syntax goes back to what I was saying about structs > classes.
//
// We could make a struct that doesn't have any functions that exist on it, like
// if we just want to make a quick struct for storing related data, but we also
// have the option to add methods onto the struct
//
// Yes, you can do functionless objects with classes by just not giving the
// class any methods. However, I like this approach more for its simplicity.
func Pickaxe.upgrade() {
	self.strength += 1;
	self.speed += 1;

	// Because pickaxe inherits from item, we can still use setLore (obviously,
	// oop still exists in my idea but it is just different than you're probably
	// used to)
	self.setLore(fmt("Strength: ${strength}\nSpeed: ${speed}\nCost: ${}",
		sqrt(Strength * Speed) 
	))

	// It could be very cool to do something with item lores & names where they
	// automatically update if something in it changed. If either of you know
	// react, I'm thinking of something like that
	self.setLore(func () {
		return fmt("Strength: ${strength}\nSpeed: ${speed}\nCost: ${}")
	}, [self.strength, self.speed])
	// Here, self.setLore would be called whenever self.strength or self.speed
	// change. The syntax of this could and probably should be different, just
	// suggesting a concept rather than how to do that concept
}

// idr how we said events should work, im just doing ewhatever lol
event playerJoin {

	// event implicitly exists inside the event, no need for making it a
	// parameter like in a function.
	event.Player.giveItem(Pickaxe(1, 1, "Pickaexe"))

	// First class functions!!! :grilled:
	Plot.players.forEach(func (Player p) {
		p.sendMessage(fmt("&c${} &fjoined!", event.Player))
	})

}

// Mapping block names to strength values. I don't have any preference on how
// this is done, I just want something to work. Putting this here rather than at
// the top of the code where it should be.
Map blockMap = {
	"grass": 1,
	"stone": 1,
	"obsidian": 9
}


event playerBreakBlock {

	// Cast the item the player is holding to a pickaxe. Again, maybe this could
	// be done using item tags in the compiler? im not sure thats a topic for
	// later.
	//
	// Im very iffy about this cast syntax, don't love it but oh well. Casting
	// should ideally be a method so that we could return an error if we can't
	// cast
	//
	// Also note that this returns a *pointer* to the item the player is
	// holding. This way, if we make changes it will modify the actual item the
	// player is holding!!! :)
	Pickaxe *pick = event.Player.heldItem.into<Pickaxe>()

	// into method would ideally return an error, but we haven't gotten to error
	// handling yet so i dont wanna make any assumptions on that. For now, we'll
	// just make `into` return null if the cast couldn't be made
	if (pick == null) {
		// Returning here is fine because potentially we could have *multiple*
		// playerBreakBlock events. In the df code it would just be the event
		// calls functions for each event we have here. 
		//
		// The wording of what i said above is weird, lmk if you're confused by
		// it i dont wanna fix it
		return
	}

	// Don't know/care how getting the name of the block that was broken works,
	// I'll do it this way lol because idc
	if (pick.strength < blockMap[event.block.name]) {
		// game action cancel event thingy
		event.cancel()
	}

	// its really dumb that a random chance makes your pick upgrade but oh well
	// idc anympore
	if (random(0,5) == 5) {
		// Note that because pick is a pointer, this upgrade function will
		// update the item the player's holding. This could be a very very
		// powerful feature of the language, and I would love to see this
		// implemented
		pick.upgrade()
	}

}

