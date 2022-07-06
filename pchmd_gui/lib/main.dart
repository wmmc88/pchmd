import 'dart:math';
import 'package:flutter_staggered_grid_view/flutter_staggered_grid_view.dart';
import 'package:flutter/material.dart';

void main() {
  runApp(const PCHMDGUIApp());
}

class PCHMDGUIApp extends StatelessWidget {
  const PCHMDGUIApp({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'PCHMD GUI',
      theme: ThemeData(
        primarySwatch: Colors.orange,
      ),
      home: const HomePage(),
    );
  }
}

class HomePage extends StatelessWidget {
  const HomePage({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Scaffold(
        appBar: AppBar(
          title: const Text('My Computer Sensors'),
          // TODO: parse from incoming data's computer name
        ),
        body: const Center(child: DataSpace()));
  }
}

class DataSpace extends StatefulWidget {
  const DataSpace({Key? key}) : super(key: key);

  @override
  State<DataSpace> createState() => _DataSpaceState();
}

class _DataSpaceState extends State<DataSpace> {
  final heightBase = 100;
  final heightOffset = 30;
  final numItems = 15;

  late final Random random;
  late final int randomColorOffset;
  late final List<double> heights;

  @override
  void initState() {
    super.initState();
    random = Random();
    randomColorOffset = random.nextInt(Colors.primaries.length);
    heights = List.generate(
        numItems,
        (index) =>
            (random.nextInt(2 * heightOffset) + heightBase - heightOffset)
                .toDouble(),
        growable: false);
  }

  @override
  Widget build(BuildContext context) {
    return MasonryGridView.count(
      crossAxisCount: 2,
      padding: const EdgeInsets.all(2.5),
      mainAxisSpacing: 5,
      crossAxisSpacing: 5,
      itemBuilder: (context, index) {
        return DataSpaceContainer(
            backgroundColor: Colors.primaries[
                (index + randomColorOffset) % Colors.primaries.length],
            verticalPadding: heights[index],
            child: Text(index.toString()));
      },
      itemCount: numItems,
    );
  }
}

class DataSpaceContainer extends StatefulWidget {
  final MaterialColor backgroundColor;
  final Widget? child;
  final double verticalPadding;

  const DataSpaceContainer(
      {Key? key,
      this.child,
      required this.backgroundColor,
      this.verticalPadding = 100})
      : super(key: key);

  @override
  State<DataSpaceContainer> createState() => _DataSpaceContainerState();
}

class _DataSpaceContainerState extends State<DataSpaceContainer> {
  @override
  Widget build(BuildContext context) {
    return Container(
        padding: EdgeInsets.symmetric(
            vertical: widget.verticalPadding, horizontal: 15),
        color: widget.backgroundColor,
        child: Center(child: widget.child));
  }
}
